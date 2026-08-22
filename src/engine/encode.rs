use crate::config::Config;
use crate::crypto::{self, GpgAuth};
use crate::entry::Meta;
use crate::profile::Profile;
use anyhow::{bail, Context, Result};
use notify_rust::Notification;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn run_encoding(
    temp_raw: &Path,
    entry_dir: &Path,
    profile: &Profile,
    do_encrypt: bool,
    config: &Config,
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
        c.arg("-n").arg("19").arg("ionice").arg("-c").arg("3").arg("ffmpeg");
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

    if let Some(ref vf) = profile.vfilter {
        if !vf.is_empty() && vf != "null" {
            cmd.arg("-vf").arg(vf);
        }
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

    if let Some(ref af) = profile.afilter {
        if !af.is_empty() && af != "null" {
            cmd.arg("-af").arg(af);
        }
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
            .body(&format!("Encoding failed for {}. Check {:?}", timestamp, encode_log))
            .show();
        bail!("FFmpeg encoding failed for {}", timestamp);
    }

    let _ = fs::remove_file(temp_raw);
    let _ = fs::remove_file(&encode_log);

    if do_encrypt {
        let auth = GpgAuth::from_config(config);
        crypto::encrypt_file(&final_out, &auth)
            .context("Failed to encrypt final video output")?;

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

    Ok(())
}

pub fn spawn_detached_encoder(
    temp_raw: &Path,
    entry_dir: &Path,
    profile_name: &str,
    do_encrypt: bool,
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

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    let child = cmd.spawn().context("Failed to spawn background encoding process")?;
    Ok(child.id())
}
