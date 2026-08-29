use crate::config::Config;
use crate::crypto::{self, GpgAuth};
use crate::entry::Meta;
use crate::overlay::{build_drawtext_filter, OverlayConfig};
use crate::profile::Profile;
use anyhow::{bail, Context, Result};
use notify_rust::Notification;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

#[allow(clippy::too_many_arguments)]
pub fn run_encoding(
    temp_raw: &Path,
    entry_dir: &Path,
    profile: &Profile,
    do_encrypt: bool,
    overlay_cfg: &OverlayConfig,
    title_opt: Option<&str>,
    config: &Config,
    denoise: bool,
    verbose: bool,
) -> Result<()> {
    let final_out = entry_dir.join("video.mkv");
    let encode_log = entry_dir.join("encode.log");
    let meta_file = entry_dir.join("meta.json");
    let note_file = entry_dir.join("note.md");
    let timestamp = entry_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut cmd = if which::which("nice").is_ok() && which::which("ionice").is_ok() {
        let mut c = Command::new("nice");
        c.arg("-n")
            .arg("19")
            .arg("ionice")
            .arg("-c")
            .arg("3")
            .arg("ffmpeg");
        c
    } else {
        Command::new("ffmpeg")
    };

    cmd.arg("-hide_banner");
    if verbose {
        cmd.arg("-loglevel").arg("info");
    } else {
        cmd.arg("-loglevel").arg("error");
    }

    cmd.arg("-i").arg(temp_raw);

    // Build video filter graph including retro overlay if enabled
    let mut vf_parts = Vec::new();
    if let Some(ref vf) = profile.vfilter {
        if !vf.is_empty() && vf != "null" {
            vf_parts.push(vf.clone());
        }
    }

    if let Some(drawtext) =
        build_drawtext_filter(&timestamp, title_opt, overlay_cfg, &profile.resolution)
    {
        vf_parts.push(drawtext);
    }

    if !vf_parts.is_empty() {
        cmd.arg("-vf").arg(vf_parts.join(","));
    }

    cmd.arg("-c:v")
        .arg(&profile.vcodec)
        .arg("-preset")
        .arg(profile.vpreset.to_string())
        .arg("-crf")
        .arg(profile.vcrf.to_string())
        .arg("-g")
        .arg("240")
        .arg("-pix_fmt")
        .arg("yuv420p");

    if let Some(ref extra) = profile.extra_flags {
        for flag in extra.split_whitespace() {
            cmd.arg(flag);
        }
    }

    if let Some(ref global_extra) = config.extra_ffmpeg_flags {
        for flag in global_extra.split_whitespace() {
            cmd.arg(flag);
        }
    }

    // Build audio filter graph including optional afftdn noise reduction
    let mut af_parts = Vec::new();
    if denoise {
        af_parts.push("afftdn=nf=-25".to_string());
    }
    if let Some(ref af) = profile.afilter {
        if !af.is_empty() && af != "null" {
            af_parts.push(af.clone());
        }
    }

    if !af_parts.is_empty() {
        cmd.arg("-af").arg(af_parts.join(","));
    }

    cmd.arg("-c:a")
        .arg(&profile.acodec)
        .arg("-ac")
        .arg(profile.achannels.to_string())
        .arg("-b:a")
        .arg(&profile.abitrate)
        .arg("-application")
        .arg("voip")
        .arg(&final_out);

    let log_file = fs::File::create(&encode_log).ok();
    if let Some(f) = log_file {
        cmd.stdout(Stdio::from(f.try_clone().unwrap()))
            .stderr(Stdio::from(f));
    } else {
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
    }

    let status = cmd.status().context("Failed to execute ffmpeg encoder")?;
    if !status.success() {
        let _ = Notification::new()
            .summary("vj error")
            .body(&format!(
                "Encoding failed for {}. Check {:?}",
                timestamp, encode_log
            ))
            .show();
        bail!("FFmpeg encoding failed for {}", timestamp);
    }

    let _ = fs::remove_file(temp_raw);
    let _ = fs::remove_file(&encode_log);

    // Generate 2x2 storyboard thumbnail for instant terminal preview
    let thumb_file = entry_dir.join("thumb.jpg");
    let _ = Command::new("ffmpeg")
        .arg("-loglevel")
        .arg("error")
        .arg("-y")
        .arg("-i")
        .arg(&final_out)
        .arg("-vf")
        .arg("thumbnail=20,scale=160:120,tile=2x2")
        .arg("-frames:v")
        .arg("1")
        .arg(&thumb_file)
        .status();

    if do_encrypt {
        let auth = GpgAuth::from_config(config);
        crypto::encrypt_file(&final_out, &auth).context("Failed to encrypt final video output")?;

        if thumb_file.exists() {
            let _ = crypto::encrypt_file(&thumb_file, &auth);
        }

        if note_file.exists() {
            let _ = crypto::encrypt_file(&note_file, &auth);
        }

        if meta_file.exists() {
            if let Ok(content) = fs::read_to_string(&meta_file) {
                if let Ok(mut meta) = serde_json::from_str::<Meta>(&content) {
                    meta.encrypted = true;
                    if let Ok(new_json) = serde_json::to_string_pretty(&meta) {
                        let _ = fs::write(&meta_file, new_json);
                    }
                }
            }
            let _ = crypto::encrypt_file(&meta_file, &auth);
        }
    }

    let _ = Notification::new()
        .summary("vj")
        .body(&format!("Finished encoding entry: {}", timestamp))
        .show();

    let file_path = if do_encrypt {
        entry_dir.join("video.mkv.gpg")
    } else {
        final_out.clone()
    };
    let _ = crate::hooks::dispatch(
        config,
        "post_encode",
        &crate::hooks::payload(
            "post_encode",
            &[
                ("entry_id", serde_json::json!(timestamp)),
                (
                    "entry_dir",
                    serde_json::json!(entry_dir.display().to_string()),
                ),
                ("file", serde_json::json!(file_path.display().to_string())),
                ("encrypted", serde_json::json!(do_encrypt)),
            ],
        ),
    );

    Ok(())
}

pub fn spawn_detached_encoder(
    temp_raw: &Path,
    entry_dir: &Path,
    profile_name: &str,
    do_encrypt: bool,
    denoise: bool,
    overlay_cfg: &OverlayConfig,
) -> Result<u32> {
    let current_exe = std::env::current_exe().context("Failed to get current executable path")?;
    let mut cmd = Command::new(current_exe);
    cmd.arg("__encode")
        .arg(temp_raw)
        .arg(entry_dir)
        .arg(profile_name);

    if do_encrypt {
        cmd.arg("--encrypt");
    }

    if denoise {
        cmd.arg("--denoise");
    }

    if overlay_cfg.enabled {
        cmd.arg("--overlay");
        cmd.arg("--overlay-style")
            .arg(format!("{:?}", overlay_cfg.style).to_lowercase());
        cmd.arg("--overlay-font").arg(&overlay_cfg.font);
        if let Some(size) = overlay_cfg.font_size {
            cmd.arg("--overlay-font-size").arg(size.to_string());
        }
        if overlay_cfg.show_title {
            cmd.arg("--overlay-title");
        }
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd
        .spawn()
        .context("Failed to spawn background encoding process")?;
    Ok(child.id())
}
