use crate::calendar;
use crate::config::Config;
use crate::engine::encode::{run_encoding, spawn_detached_encoder};
use crate::entry::Meta;
use crate::overlay::{OverlayConfig, OverlayStyle};
use crate::profile;
use anyhow::{bail, Context, Result};
use dialoguer::{Confirm, Input};
use std::fs;
use std::io::IsTerminal;
use std::process::{Command, Stdio};

pub struct RecordOptions {
    pub profile: Option<String>,
    pub encrypt: Option<bool>,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub note: bool,
    pub interactive: bool,
    pub verbose: bool,
    pub wait: bool,
    pub denoise: Option<bool>,
    pub overlay: Option<bool>,
    pub overlay_style: Option<String>,
    pub overlay_font: Option<String>,
    pub overlay_font_size: Option<u32>,
    pub overlay_title: Option<bool>,
}

pub fn execute_record(opts: RecordOptions, config: &Config) -> Result<()> {
    // Check prerequisites
    if which::which("ffmpeg").is_err() {
        bail!("ffmpeg is required but not found in PATH.");
    }
    if which::which("ffplay").is_err() {
        bail!("ffplay is required for live recording preview but not found in PATH.");
    }

    config.ensure_directories()?;

    let profile_name = opts
        .profile
        .as_deref()
        .unwrap_or(&config.default_profile);

    let (resolved_name, profile_spec) =
        profile::resolve_profile(profile_name, &config.profiles);

    let do_encrypt = opts.encrypt.unwrap_or(config.auto_encrypt);
    let cal_sys = config.calendar_system();
    let timestamp = calendar::get_current_timestamp(cal_sys);

    let entry_folder = config.entries_path().join(&timestamp);
    let temp_raw = config.temp_path().join(format!("raw_{}.mkv", timestamp));
    let meta_file = entry_folder.join("meta.json");
    let note_file = entry_folder.join("note.md");

    fs::create_dir_all(&entry_folder)
        .with_context(|| format!("Failed to create entry folder {:?}", entry_folder))?;

    println!(
        "Recording [{}: {}@{}fps] -> {} (Close preview or press 'q' to stop)",
        resolved_name, profile_spec.resolution, profile_spec.fps, timestamp
    );

    // Step 1: Capture with live preview via pipeline
    let mut ffmpeg_cmd = Command::new("ffmpeg");
    ffmpeg_cmd.arg("-hide_banner");

    if opts.verbose {
        ffmpeg_cmd.arg("-loglevel").arg("info");
    } else {
        ffmpeg_cmd.arg("-loglevel").arg("quiet");
    }

    ffmpeg_cmd
        .arg("-f")
        .arg("v4l2")
        .arg("-framerate")
        .arg(profile_spec.fps.to_string())
        .arg("-video_size")
        .arg(&profile_spec.resolution)
        .arg("-i")
        .arg(&config.camera_dev)
        .arg("-f")
        .arg("pulse")
        .arg("-i")
        .arg(&config.audio_src)
        .arg("-filter_complex")
        .arg("[0:v]split=2[rec_v][preview_v]")
        .arg("-map")
        .arg("[rec_v]")
        .arg("-map")
        .arg("1:a")
        .arg("-c:v")
        .arg("libx264")
        .arg("-preset")
        .arg("ultrafast")
        .arg("-crf")
        .arg("17")
        .arg("-c:a")
        .arg("pcm_s16le")
        .arg(&temp_raw)
        .arg("-map")
        .arg("[preview_v]")
        .arg("-c:v")
        .arg("rawvideo")
        .arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-f")
        .arg("nut")
        .arg("-");

    ffmpeg_cmd.stdout(Stdio::piped());
    if !opts.verbose {
        ffmpeg_cmd.stderr(Stdio::null());
    }

    let mut ffmpeg_child = ffmpeg_cmd
        .spawn()
        .context("Failed to start ffmpeg capture process")?;

    let ffmpeg_out = ffmpeg_child
        .stdout
        .take()
        .context("Failed to capture ffmpeg output pipe")?;

    let mut ffplay_cmd = Command::new("ffplay");
    ffplay_cmd.arg("-hide_banner");

    if opts.verbose {
        ffplay_cmd.arg("-loglevel").arg("info");
    } else {
        ffplay_cmd.arg("-loglevel").arg("quiet").arg("-nostats");
    }

    ffplay_cmd
        .arg("-window_title")
        .arg(format!("vj recording ({})", timestamp))
        .arg("-f")
        .arg("nut")
        .arg("-");

    ffplay_cmd.stdin(Stdio::from(ffmpeg_out));
    if !opts.verbose {
        ffplay_cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let mut ffplay_child = ffplay_cmd
        .spawn()
        .context("Failed to start ffplay preview process")?;

    // Wait for preview window to close
    let _ = ffplay_child.wait();

    // Terminate/wait for ffmpeg
    let _ = ffmpeg_child.kill();
    let _ = ffmpeg_child.wait();

    if !temp_raw.exists() || fs::metadata(&temp_raw).map(|m| m.len()).unwrap_or(0) == 0 {
        println!("Recording cancelled or no data captured.");
        let _ = fs::remove_dir_all(&entry_folder);
        let _ = fs::remove_file(&temp_raw);
        return Ok(());
    }

    println!("Capture finished.");

    // Step 2: Metadata & Notes
    let mut custom_title = opts.title.clone();
    let mut custom_tags = opts.tags.clone();
    let mut open_note = opts.note;

    if opts.interactive && std::io::stdin().is_terminal() {
        let input_title: String = Input::new()
            .with_prompt("Title [press enter for default]")
            .allow_empty(true)
            .interact_text()?;
        if !input_title.trim().is_empty() {
            custom_title = Some(input_title.trim().to_string());
        }

        let input_tags: String = Input::new()
            .with_prompt("Tags (comma-separated, optional)")
            .allow_empty(true)
            .interact_text()?;
        if !input_tags.trim().is_empty() {
            custom_tags = Some(input_tags.trim().to_string());
        }

        open_note = Confirm::new()
            .with_prompt(format!("Write note in {}?", config.editor))
            .default(false)
            .interact()?;
    }

    let final_title = custom_title.unwrap_or_else(|| format!("Entry {}", timestamp));
    let parsed_tags: Vec<String> = custom_tags
        .as_deref()
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if open_note {
        let note_content = format!(
            "# {}\n*Date: {}*\n*Tags: {}*\n\n",
            final_title,
            timestamp,
            parsed_tags.join(", ")
        );
        let _ = fs::write(&note_file, note_content);

        let editor = &config.editor;
        let mut ed_cmd = Command::new(editor);
        ed_cmd.arg(&note_file);
        let _ = ed_cmd.status();
    }

    let meta = Meta {
        timestamp: timestamp.clone(),
        calendar: Some(cal_sys.to_string()),
        title: final_title.clone(),
        original_filename: None,
        imported: Some(false),
        profile: resolved_name.clone(),
        resolution: Some(profile_spec.resolution.clone()),
        fps: Some(profile_spec.fps),
        tags: parsed_tags,
        encrypted: false,
    };

    let meta_json = serde_json::to_string_pretty(&meta).context("Failed to serialize meta.json")?;
    fs::write(&meta_file, meta_json).context("Failed to write meta.json")?;

    // Determine overlay settings
    let overlay_enabled = opts.overlay.unwrap_or(config.retro_overlay);
    let overlay_style: OverlayStyle = opts
        .overlay_style
        .as_deref()
        .unwrap_or(&config.overlay_style)
        .parse()
        .unwrap_or_default();
    let overlay_font = opts
        .overlay_font
        .clone()
        .unwrap_or_else(|| config.overlay_font.clone());
    let overlay_font_size = opts.overlay_font_size.or(config.overlay_font_size);
    let overlay_title = opts.overlay_title.unwrap_or(config.overlay_show_title);

    let overlay_cfg = OverlayConfig {
        enabled: overlay_enabled,
        style: overlay_style,
        font: overlay_font,
        font_size: overlay_font_size,
        show_title: overlay_title,
    };

    let denoise_enabled = opts.denoise.unwrap_or(config.denoise);

    // Step 3: Compression + Encryption
    if opts.wait {
        println!("Encoding in foreground...");
        run_encoding(
            &temp_raw,
            &entry_folder,
            &profile_spec,
            do_encrypt,
            &overlay_cfg,
            Some(&final_title),
            config,
            denoise_enabled,
            opts.verbose,
        )?;
        println!("[✓] Entry saved and encoded to {}", entry_folder.display());
    } else {
        match spawn_detached_encoder(
            &temp_raw,
            &entry_folder,
            &resolved_name,
            do_encrypt,
            denoise_enabled,
            &overlay_cfg,
        ) {
            Ok(pid) => {
                println!(
                    "Encoding in background (PID: {}). Entry saved to {}",
                    pid,
                    entry_folder.display()
                );
            }
            Err(e) => {
                eprintln!("Failed to spawn background encoder: {}. Running in foreground...", e);
                run_encoding(
                    &temp_raw,
                    &entry_folder,
                    &profile_spec,
                    do_encrypt,
                    &overlay_cfg,
                    Some(&final_title),
                    config,
                    denoise_enabled,
                    opts.verbose,
                )?;
            }
        }
    }

    Ok(())
}
