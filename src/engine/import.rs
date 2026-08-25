use crate::calendar::{self, CalendarSystem};
use crate::config::Config;
use crate::crypto::{self, GpgAuth};
use crate::entry::Meta;
use crate::overlay::{build_drawtext_filter, OverlayConfig, OverlayStyle};
use crate::profile;
use anyhow::{bail, Context, Result};
use chrono::DateTime;
use dialoguer::{Confirm, Input};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub struct ImportOptions {
    pub files: Vec<PathBuf>,
    pub profile: Option<String>,
    pub encrypt: Option<bool>,
    pub title: Option<String>,
    pub tags: Option<String>,
    pub note: bool,
    pub interactive: bool,
    pub keep: bool,
    pub verbose: bool,
    pub overlay: Option<bool>,
    pub overlay_style: Option<String>,
    pub overlay_font: Option<String>,
    pub overlay_font_size: Option<u32>,
    pub overlay_title: Option<bool>,
}

pub fn get_video_creation_timestamp(file: &Path, cal: CalendarSystem) -> String {
    if which::which("ffprobe").is_ok() {
        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-show_entries")
            .arg("format_tags=creation_time:stream_tags=creation_time")
            .arg("-of")
            .arg("default=noprint_wrappers=1:nokey=1")
            .arg(file)
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                let trimmed = line.trim();
                if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
                    return calendar::format_epoch_timestamp(dt.timestamp(), cal);
                }
            }
        }
    }

    if let Ok(m) = fs::metadata(file) {
        if let Ok(modified) = m.modified() {
            if let Ok(dur) = modified.duration_since(std::time::UNIX_EPOCH) {
                return calendar::format_epoch_timestamp(dur.as_secs() as i64, cal);
            }
        }
    }

    calendar::get_current_timestamp(cal)
}

pub fn preview_inbox_item(file: &Path, cal: CalendarSystem) -> Result<()> {
    if !file.exists() {
        bail!("File not found: {:?}", file);
    }

    println!("================ INBOX VIDEO PREVIEW ================");
    println!("File:         {}", file.file_name().unwrap_or_default().to_string_lossy());
    if let Ok(m) = fs::metadata(file) {
        let sz = m.len();
        let size_str = if sz >= 1024 * 1024 * 1024 {
            format!("{:.1} GB", sz as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if sz >= 1024 * 1024 {
            format!("{:.1} MB", sz as f64 / (1024.0 * 1024.0))
        } else if sz >= 1024 {
            format!("{:.1} KB", sz as f64 / 1024.0)
        } else {
            format!("{} B", sz)
        };
        println!("Size:         {}", size_str);
    }
    println!("Path:         {}", file.display());
    println!();

    if which::which("ffprobe").is_ok() {
        let output = Command::new("ffprobe")
            .arg("-v")
            .arg("error")
            .arg("-select_streams")
            .arg("v:0")
            .arg("-show_entries")
            .arg("stream=width,height,r_frame_rate,codec_name:format=duration")
            .arg("-of")
            .arg("default=noprint_wrappers=1")
            .arg(file)
            .output();

        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            let mut width = "";
            let mut height = "";
            let mut codec = "";
            let mut fps_str = "".to_string();
            let mut duration_str = "".to_string();

            for line in s.lines() {
                if let Some(w) = line.strip_prefix("width=") {
                    width = w;
                } else if let Some(h) = line.strip_prefix("height=") {
                    height = h;
                } else if let Some(c) = line.strip_prefix("codec_name=") {
                    codec = c;
                } else if let Some(r) = line.strip_prefix("r_frame_rate=") {
                    if let Some((num, den)) = r.split_once('/') {
                        if let (Ok(n), Ok(d)) = (num.parse::<f64>(), den.parse::<f64>()) {
                            if d > 0.0 {
                                fps_str = format!("{:.0}", n / d);
                            }
                        }
                    }
                } else if let Some(dur) = line.strip_prefix("duration=") {
                    if let Ok(sec) = dur.parse::<f64>() {
                        let total = sec as u64;
                        let h = total / 3600;
                        let m = (total % 3600) / 60;
                        let s = total % 60;
                        duration_str = format!("{:02}:{:02}:{:02}", h, m, s);
                    }
                }
            }

            if !width.is_empty() && !height.is_empty() {
                println!("Resolution:   {}x{} @ {}fps", width, height, fps_str);
                println!("Codec:        {}", codec);
                println!("Duration:     {}", duration_str);
            }
        }
    }

    let detected = get_video_creation_timestamp(file, cal);
    println!("Detected Date: {}", detected);
    Ok(())
}

pub fn execute_import(opts: ImportOptions, config: &Config) -> Result<()> {
    if which::which("ffmpeg").is_err() {
        bail!("ffmpeg is required for importing videos but was not found.");
    }

    config.ensure_directories()?;
    let cal_sys = config.calendar_system();

    let mut files_to_import = opts.files.clone();

    if files_to_import.is_empty() {
        let inbox = config.inbox_path();
        let mut candidates = Vec::new();

        if inbox.exists() {
            for entry in fs::read_dir(&inbox)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                        let ext_lower = ext.to_lowercase();
                        if matches!(
                            ext_lower.as_str(),
                            "mp4" | "mkv" | "mov" | "webm" | "3gp" | "avi" | "m4v" | "ts"
                        ) {
                            candidates.push(path);
                        }
                    }
                }
            }
        }

        if candidates.is_empty() {
            println!("No video files found in inbox: {}", inbox.display());
            println!("You can drop video files into {} or run 'vj inbox-server' to upload from your phone.", inbox.display());
            return Ok(());
        }

        if std::io::stdin().is_terminal() && which::which("fzf").is_ok() {
            let current_exe = std::env::current_exe()?;
            let preview_cmd = format!("{} preview-inbox {{}}", current_exe.display());

            let mut fzf_cmd = Command::new("fzf");
            fzf_cmd
                .arg("-m")
                .arg("--prompt=Select Videos to Import (Tab to multi-select) > ")
                .arg("--header=[Tab: Multi-select | Ctrl-A: Select All | Enter: Import | Esc: Cancel]")
                .arg(format!("--preview={}", preview_cmd))
                .arg("--preview-window=right:55%:wrap")
                .arg("--bind=ctrl-a:select-all,ctrl-j:down,ctrl-k:up,ctrl-d:page-down,ctrl-u:page-up")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped());

            let mut child = fzf_cmd.spawn().context("Failed to run fzf")?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                for c in &candidates {
                    writeln!(stdin, "{}", c.display())?;
                }
            }

            let output = child.wait_with_output()?;
            if output.status.success() {
                let selected = String::from_utf8_lossy(&output.stdout);
                for line in selected.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        files_to_import.push(PathBuf::from(trimmed));
                    }
                }
            } else {
                println!("Import cancelled.");
                return Ok(());
            }
        } else {
            files_to_import = candidates;
        }
    }

    if files_to_import.is_empty() {
        println!("No files selected for import.");
        return Ok(());
    }

    let profile_name = opts
        .profile
        .as_deref()
        .unwrap_or(&config.default_profile);

    let (resolved_name, profile_spec) =
        profile::resolve_profile(profile_name, &config.profiles);

    let do_encrypt = opts.encrypt.unwrap_or(config.auto_encrypt);
    let auth = GpgAuth::from_config(config);

    // Determine overlay configuration
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

    println!(
        "Importing {} video file(s) [Profile: {}]...",
        files_to_import.len(),
        resolved_name
    );

    let total = files_to_import.len();
    for (idx, raw_file) in files_to_import.iter().enumerate() {
        if !raw_file.exists() {
            eprintln!("Skipping missing file: {}", raw_file.display());
            continue;
        }

        let bname = raw_file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let mut timestamp = get_video_creation_timestamp(raw_file, cal_sys);
        let mut entry_folder = config.entries_path().join(&timestamp);
        let mut count = 1;
        while entry_folder.exists() {
            timestamp = format!("{}_{}", get_video_creation_timestamp(raw_file, cal_sys), count);
            entry_folder = config.entries_path().join(&timestamp);
            count += 1;
        }

        fs::create_dir_all(&entry_folder)?;

        let final_out = entry_folder.join("video.mkv");
        let meta_file = entry_folder.join("meta.json");
        let note_file = entry_folder.join("note.md");

        let mut this_title = opts.title.clone();
        let mut this_tags = opts.tags.clone();
        let mut this_open_note = opts.note;

        if opts.interactive && std::io::stdin().is_terminal() {
            println!("\n--- Configuring: {} (Date: {}) [{}/{}] ---", bname, timestamp, idx + 1, total);
            let input_title: String = Input::new()
                .with_prompt(format!("Title [press enter for 'Imported: {}']", bname))
                .allow_empty(true)
                .interact_text()?;
            if !input_title.trim().is_empty() {
                this_title = Some(input_title.trim().to_string());
            }

            let input_tags: String = Input::new()
                .with_prompt("Tags (comma-separated, optional)")
                .allow_empty(true)
                .interact_text()?;
            if !input_tags.trim().is_empty() {
                this_tags = Some(input_tags.trim().to_string());
            }

            this_open_note = Confirm::new()
                .with_prompt(format!("Write note in {}?", config.editor))
                .default(false)
                .interact()?;
        }

        let title_val = this_title.unwrap_or_else(|| format!("Imported: {}", bname));
        let parsed_tags: Vec<String> = this_tags
            .as_deref()
            .unwrap_or("")
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if this_open_note {
            let note_content = format!(
                "# {}\n*Imported: {}*\n*Date: {}*\n*Tags: {}*\n\n",
                title_val,
                bname,
                timestamp,
                parsed_tags.join(", ")
            );
            let _ = fs::write(&note_file, note_content);
            let mut ed_cmd = Command::new(&config.editor);
            ed_cmd.arg(&note_file);
            let _ = ed_cmd.status();
        }

        let meta = Meta {
            timestamp: timestamp.clone(),
            calendar: Some(cal_sys.to_string()),
            title: title_val.clone(),
            original_filename: Some(bname.clone()),
            imported: Some(true),
            profile: resolved_name.clone(),
            resolution: Some(profile_spec.resolution.clone()),
            fps: Some(profile_spec.fps),
            tags: parsed_tags,
            encrypted: false,
        };

        let meta_json = serde_json::to_string_pretty(&meta)?;
        fs::write(&meta_file, meta_json)?;

        println!("Compressing [{}] {} -> {}...", resolved_name, bname, timestamp);

        let mut cmd = if which::which("nice").is_ok() && which::which("ionice").is_ok() {
            let mut c = Command::new("nice");
            c.arg("-n").arg("19").arg("ionice").arg("-c").arg("3").arg("ffmpeg");
            c
        } else {
            Command::new("ffmpeg")
        };

        cmd.arg("-hide_banner");
        if opts.verbose {
            cmd.arg("-loglevel").arg("info");
        } else {
            cmd.arg("-loglevel").arg("quiet");
        }

        cmd.arg("-i").arg(raw_file);

        // Build video filter graph with retro overlay
        let mut vf_parts = Vec::new();
        if let Some(ref vf) = profile_spec.vfilter {
            if !vf.is_empty() && vf != "null" {
                vf_parts.push(vf.clone());
            }
        }

        if let Some(drawtext) = build_drawtext_filter(&timestamp, Some(&title_val), &overlay_cfg, &profile_spec.resolution) {
            vf_parts.push(drawtext);
        }

        if !vf_parts.is_empty() {
            cmd.arg("-vf").arg(vf_parts.join(","));
        }

        cmd.arg("-c:v")
            .arg(&profile_spec.vcodec)
            .arg("-preset")
            .arg(profile_spec.vpreset.to_string())
            .arg("-crf")
            .arg(profile_spec.vcrf.to_string())
            .arg("-g")
            .arg("240")
            .arg("-pix_fmt")
            .arg("yuv420p");

        if let Some(ref extra) = profile_spec.extra_flags {
            for flag in extra.split_whitespace() {
                cmd.arg(flag);
            }
        }

        if let Some(ref global_extra) = config.extra_ffmpeg_flags {
            for flag in global_extra.split_whitespace() {
                cmd.arg(flag);
            }
        }

        if let Some(ref af) = profile_spec.afilter {
            if !af.is_empty() && af != "null" {
                cmd.arg("-af").arg(af);
            }
        }

        cmd.arg("-c:a")
            .arg(&profile_spec.acodec)
            .arg("-ac")
            .arg(profile_spec.achannels.to_string())
            .arg("-b:a")
            .arg(&profile_spec.abitrate)
            .arg("-application")
            .arg("voip")
            .arg(&final_out);

        if !opts.verbose {
            cmd.stdout(Stdio::null()).stderr(Stdio::null());
        }

        let status = cmd.status().context("FFmpeg import encoding failed")?;
        if !status.success() {
            eprintln!("Failed to encode {}", bname);
            continue;
        }

        if do_encrypt {
            crypto::encrypt_file(&final_out, &auth)?;
            if note_file.exists() {
                let _ = crypto::encrypt_file(&note_file, &auth);
            }
            if meta_file.exists() {
                let mut m = meta.clone();
                m.encrypted = true;
                if let Ok(j) = serde_json::to_string_pretty(&m) {
                    let _ = fs::write(&meta_file, j);
                }
                let _ = crypto::encrypt_file(&meta_file, &auth);
            }
        }

        if !opts.keep {
            let _ = fs::remove_file(raw_file);
        }

        println!("[✓] Imported {} -> {}", bname, entry_folder.display());
    }

    use notify_rust::Notification;
    let _ = Notification::new()
        .summary("vj")
        .body(&format!("Finished importing {} video(s)", total))
        .show();

    println!("All imports completed.");
    Ok(())
}
