use crate::config::Config;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

pub struct GpgAuth {
    pub key_file: Option<String>,
    pub passphrase: Option<String>,
}

impl GpgAuth {
    pub fn from_config(config: &Config) -> Self {
        let env_pass = std::env::var("VJ_PASSPHRASE")
            .or_else(|_| std::env::var("DIARY_PASSPHRASE"))
            .ok();

        let pass = config.passphrase.clone().or(env_pass);
        let key = config.key_file_path().and_then(|p| {
            if p.exists() {
                Some(p.to_string_lossy().to_string())
            } else {
                None
            }
        });

        Self {
            key_file: key,
            passphrase: pass,
        }
    }

    pub fn apply_to_cmd(&self, cmd: &mut Command) {
        if let Some(ref kf) = self.key_file {
            cmd.arg("--pinentry-mode")
                .arg("loopback")
                .arg("--passphrase-file")
                .arg(kf);
        } else if let Some(ref pass) = self.passphrase {
            cmd.arg("--pinentry-mode")
                .arg("loopback")
                .arg("--passphrase")
                .arg(pass);
        }
    }

    pub fn has_auth(&self) -> bool {
        self.key_file.is_some() || self.passphrase.is_some()
    }
}

pub fn encrypt_file<P: AsRef<Path>>(path: P, auth: &GpgAuth) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }

    let mut cmd = Command::new("gpg");
    cmd.arg("--batch").arg("--yes");
    auth.apply_to_cmd(&mut cmd);
    cmd.arg("--symmetric")
        .arg("--cipher-algo")
        .arg("AES256")
        .arg(path);

    cmd.stdout(Stdio::null()).stderr(Stdio::null());

    let status = cmd.status().context("Failed to run gpg encryption")?;
    if !status.success() {
        bail!("GPG encryption failed for {:?}", path);
    }

    let _ = fs::remove_file(path);
    Ok(())
}

pub fn decrypt_file<P1: AsRef<Path>, P2: AsRef<Path>>(
    encrypted_path: P1,
    out_path: P2,
    auth: &GpgAuth,
) -> Result<()> {
    let enc = encrypted_path.as_ref();
    let out = out_path.as_ref();

    if !enc.exists() {
        return Ok(());
    }

    let mut cmd = Command::new("gpg");
    cmd.arg("--batch").arg("--yes");
    auth.apply_to_cmd(&mut cmd);
    cmd.arg("--decrypt").arg(enc);

    let out_file =
        fs::File::create(out).context("Failed to create destination file for decryption")?;
    cmd.stdout(Stdio::from(out_file)).stderr(Stdio::null());

    let status = cmd.status().context("Failed to run gpg decryption")?;
    if !status.success() {
        let _ = fs::remove_file(out);
        bail!("GPG decryption failed for {:?}", enc);
    }

    let _ = fs::remove_file(enc);
    Ok(())
}

pub fn decrypt_to_string<P: AsRef<Path>>(encrypted_path: P, auth: &GpgAuth) -> Result<String> {
    let enc = encrypted_path.as_ref();
    if !enc.exists() {
        bail!("File not found: {:?}", enc);
    }

    let mut cmd = Command::new("gpg");
    cmd.arg("--batch");
    auth.apply_to_cmd(&mut cmd);
    cmd.arg("--decrypt").arg(enc);
    cmd.stderr(Stdio::null());

    let output = cmd.output().context("Failed to execute gpg")?;
    if output.status.success() {
        let s = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(s)
    } else {
        bail!("GPG decryption failed");
    }
}
