use crate::config::Config;
use crate::crypto::GpgAuth;
use crate::entry::{find_entry, load_entries};
use anyhow::{bail, Context, Result};
use std::io::IsTerminal;
use std::process::{Command, Stdio};

pub fn execute_play(target: Option<String>, verbose: bool, config: &Config) -> Result<()> {
    let entries = load_entries(&config.entries_path(), &config.temp_path())?;
    if entries.is_empty() {
        println!("No entries found in {}", config.entries_path().display());
        return Ok(());
    }

    let mut selected_id = target;

    // If no target given, run interactive fzf browser
    if selected_id.is_none() {
        if std::io::stdin().is_terminal() && which::which("fzf").is_ok() {
            let current_exe = std::env::current_exe()?;
            let preview_cmd = format!("{} preview {{1}}", current_exe.display());
            let peek_cmd = format!("{} __peek {{1}}", current_exe.display());

            let mut fzf_cmd = Command::new("fzf");
            fzf_cmd
                .arg("--prompt=vj play > ")
                .arg("--header=[Enter: Play | Ctrl-P / Space: Peek Video | C-u/C-d: Scroll | Esc: Quit]")
                .arg(format!("--preview={}", preview_cmd))
                .arg("--preview-window=right:55%:wrap")
                .arg(format!("--bind=ctrl-p:execute-silent({}),space:execute-silent({})", peek_cmd, peek_cmd))
                .arg("--bind=ctrl-j:down,ctrl-k:up,ctrl-d:page-down,ctrl-u:page-up,ctrl-y:preview-up,ctrl-e:preview-down")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped());

            let mut child = fzf_cmd.spawn().context("Failed to run fzf")?;
            if let Some(mut stdin) = child.stdin.take() {
                use std::io::Write;
                for e in &entries {
                    let status = if e.is_encrypted { "[GPG]" } else { "[RAW]" };
                    writeln!(
                        stdin,
                        "{:<22} {:<10} {:<8} {:<32} {}",
                        e.id,
                        status,
                        e.formatted_size(),
                        e.title(),
                        e.tags_string()
                    )?;
                }
            }

            let output = child.wait_with_output()?;
            if output.status.success() {
                let line = String::from_utf8_lossy(&output.stdout);
                if let Some(first_word) = line.split_whitespace().next() {
                    selected_id = Some(first_word.to_string());
                }
            } else {
                return Ok(());
            }
        } else {
            selected_id = entries.first().map(|e| e.id.clone());
        }
    }

    let target_id = match selected_id {
        Some(id) => id,
        None => return Ok(()),
    };

    let entry = match find_entry(&entries, &target_id) {
        Some(e) => e,
        None => {
            bail!("Entry not found: {}", target_id);
        }
    };

    if which::which("mpv").is_err() {
        bail!("mpv is required for playback but was not found in PATH.");
    }

    crate::hooks::dispatch_entry(
        config,
        "pre_play",
        &entry.id,
        &entry.dir,
        entry.meta.as_ref().map(|m| m.profile.as_str()),
        entry.meta.as_ref().map(|m| m.title.as_str()),
        &entry
            .meta
            .as_ref()
            .map(|m| m.tags.clone())
            .unwrap_or_default(),
        entry.is_encrypted,
    )?;

    let auth = GpgAuth::from_config(config);
    let v_gpg = entry.dir.join("video.mkv.gpg");
    let v_plain = entry.dir.join("video.mkv");

    let mut mpv_args = vec![format!("--title=vj - {}", entry.id)];
    if !verbose {
        mpv_args.push("--msg-level=all=no".to_string());
        mpv_args.push("--really-quiet".to_string());
    }

    if v_gpg.exists() {
        // Encrypted zero-disk-leak streaming pipeline: gpg --decrypt | mpv -
        let mut gpg_cmd = Command::new("gpg");
        gpg_cmd.arg("--batch");
        auth.apply_to_cmd(&mut gpg_cmd);
        gpg_cmd.arg("--decrypt").arg(&v_gpg);
        gpg_cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        if let Ok(mut gpg_child) = gpg_cmd.spawn() {
            if let Some(gpg_out) = gpg_child.stdout.take() {
                let mut mpv_cmd = Command::new("mpv");
                for arg in &mpv_args {
                    mpv_cmd.arg(arg);
                }
                mpv_cmd.arg("-");
                mpv_cmd.stdin(Stdio::from(gpg_out));

                if let Ok(mut mpv_child) = mpv_cmd.spawn() {
                    let _ = mpv_child.wait();
                    let _ = gpg_child.wait();
                    return Ok(());
                }
            }
            let _ = gpg_child.kill();
        }

        // Fallback to interactive gpg
        let mut fallback_gpg = Command::new("gpg");
        fallback_gpg.arg("--decrypt").arg(&v_gpg);
        fallback_gpg.stdout(Stdio::piped());

        let mut gpg_child = fallback_gpg
            .spawn()
            .context("Failed to run interactive gpg decryption")?;

        if let Some(gpg_out) = gpg_child.stdout.take() {
            let mut mpv_cmd = Command::new("mpv");
            for arg in &mpv_args {
                mpv_cmd.arg(arg);
            }
            mpv_cmd.arg("-");
            mpv_cmd.stdin(Stdio::from(gpg_out));

            let mut mpv_child = mpv_cmd.spawn().context("Failed to start mpv player")?;
            let _ = mpv_child.wait();
            let _ = gpg_child.wait();
        }
    } else if v_plain.exists() {
        let mut mpv_cmd = Command::new("mpv");
        for arg in &mpv_args {
            mpv_cmd.arg(arg);
        }
        mpv_cmd.arg(&v_plain);
        let mut child = mpv_cmd.spawn().context("Failed to start mpv player")?;
        let _ = child.wait();
    } else if entry.is_encoding {
        println!("Video file is still compressing in the background.");
    } else {
        bail!("Video file missing for entry {}", entry.id);
    }

    crate::hooks::dispatch_entry(
        config,
        "post_play",
        &entry.id,
        &entry.dir,
        entry.meta.as_ref().map(|m| m.profile.as_str()),
        entry.meta.as_ref().map(|m| m.title.as_str()),
        &entry
            .meta
            .as_ref()
            .map(|m| m.tags.clone())
            .unwrap_or_default(),
        entry.is_encrypted,
    )?;

    Ok(())
}

pub fn execute_peek(target: &str, verbose: bool, config: &Config) -> Result<()> {
    let entries = load_entries(&config.entries_path(), &config.temp_path())?;
    let entry = match find_entry(&entries, target) {
        Some(e) => e,
        None => return Ok(()),
    };

    if which::which("mpv").is_err() {
        return Ok(());
    }

    let auth = GpgAuth::from_config(config);
    let v_gpg = entry.dir.join("video.mkv.gpg");
    let v_plain = entry.dir.join("video.mkv");

    let mut mpv_args = vec![
        format!("--title=vj peek - {}", entry.id),
        "--no-audio".to_string(),
        "--loop-file=inf".to_string(),
        "--ontop".to_string(),
        "--no-border".to_string(),
        "--geometry=30%x30%-20-20".to_string(),
        "--autofit=360x270".to_string(),
        "--really-quiet".to_string(),
    ];

    if !verbose {
        mpv_args.push("--msg-level=all=no".to_string());
    }

    if v_gpg.exists() {
        let mut gpg_cmd = Command::new("gpg");
        gpg_cmd.arg("--batch");
        auth.apply_to_cmd(&mut gpg_cmd);
        gpg_cmd.arg("--decrypt").arg(&v_gpg);
        gpg_cmd.stdout(Stdio::piped()).stderr(Stdio::null());

        if let Ok(mut gpg_child) = gpg_cmd.spawn() {
            if let Some(gpg_out) = gpg_child.stdout.take() {
                let mut mpv_cmd = Command::new("mpv");
                for arg in &mpv_args {
                    mpv_cmd.arg(arg);
                }
                mpv_cmd.arg("-");
                mpv_cmd.stdin(Stdio::from(gpg_out));

                if let Ok(mut mpv_child) = mpv_cmd.spawn() {
                    let _ = mpv_child.wait();
                    let _ = gpg_child.wait();
                    return Ok(());
                }
            }
            let _ = gpg_child.kill();
        }
    } else if v_plain.exists() {
        let mut mpv_cmd = Command::new("mpv");
        for arg in &mpv_args {
            mpv_cmd.arg(arg);
        }
        mpv_cmd.arg(&v_plain);
        let mut child = mpv_cmd.spawn().context("Failed to start mpv player")?;
        let _ = child.wait();
    }

    Ok(())
}

pub fn execute_random(verbose: bool, config: &Config) -> Result<()> {
    let entries = load_entries(&config.entries_path(), &config.temp_path())?;
    if entries.is_empty() {
        println!("No entries found.");
        return Ok(());
    }

    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;

    let idx = seed % entries.len();
    let chosen = &entries[idx];
    println!("Playing random entry: {}", chosen.id);
    execute_play(Some(chosen.id.clone()), verbose, config)
}
