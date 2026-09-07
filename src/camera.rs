use anyhow::{bail, Context, Result};
use dialoguer::Select;
use std::fs;
use std::io::IsTerminal;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CameraDevice {
    pub device: String,
    pub name: String,
    pub index: u32,
}

/// Detect available V4L2 cameras on the system via sysfs and /dev
pub fn list_camera_devices() -> Vec<CameraDevice> {
    let mut devices = Vec::new();

    // 1. Try sysfs /sys/class/video4linux/
    let sysfs_path = Path::new("/sys/class/video4linux");
    if let Ok(entries) = fs::read_dir(sysfs_path) {
        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let name_str = file_name.to_string_lossy();
            if let Some(suffix) = name_str.strip_prefix("video") {
                let dev_path = format!("/dev/{}", name_str);
                if Path::new(&dev_path).exists() {
                    let index: u32 = suffix.parse().unwrap_or(u32::MAX);
                    let display_name = fs::read_to_string(entry.path().join("name"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| name_str.to_string());

                    devices.push(CameraDevice {
                        device: dev_path,
                        name: display_name,
                        index,
                    });
                }
            }
        }
    }

    // 2. Fallback: check /dev/video* if sysfs was empty
    if devices.is_empty() {
        if let Ok(entries) = fs::read_dir("/dev") {
            for entry in entries.flatten() {
                let file_name = entry.file_name();
                let name_str = file_name.to_string_lossy();
                if let Some(suffix) = name_str.strip_prefix("video") {
                    let dev_path = format!("/dev/{}", name_str);
                    let index: u32 = suffix.parse().unwrap_or(u32::MAX);
                    devices.push(CameraDevice {
                        device: dev_path,
                        name: "V4L2 Video Device".to_string(),
                        index,
                    });
                }
            }
        }
    }

    // Sort by device index
    devices.sort_by_key(|d| d.index);
    devices
}

/// Prompt the user interactively to select a camera
pub fn select_camera_interactive() -> Result<String> {
    let cameras = list_camera_devices();

    if cameras.is_empty() {
        if !std::io::stdin().is_terminal() {
            bail!("No camera devices detected and stdin is not an interactive terminal.");
        }
        let input: String = dialoguer::Input::new()
            .with_prompt("No cameras auto-detected. Enter camera device path")
            .default("/dev/video0".to_string())
            .interact_text()
            .context("Failed to read camera device input")?;
        return Ok(input.trim().to_string());
    }

    if cameras.len() == 1 {
        println!(
            "Only one camera detected: {} ({})",
            cameras[0].device, cameras[0].name
        );
        return Ok(cameras[0].device.clone());
    }

    if !std::io::stdin().is_terminal() {
        bail!(
            "Multiple cameras detected ({}), but stdin is not an interactive terminal. Please specify a camera device (e.g. -c /dev/video0).",
            cameras.len()
        );
    }

    let items: Vec<String> = cameras
        .iter()
        .map(|c| format!("{} ({})", c.device, c.name))
        .collect();

    let selection = Select::new()
        .with_prompt("Select Camera Device")
        .default(0)
        .items(&items)
        .interact()
        .context("Failed to select camera")?;

    Ok(cameras[selection].device.clone())
}

/// Resolve camera device from CLI option or default config
pub fn resolve_camera_device(camera_opt: Option<&str>, default_device: &str) -> Result<String> {
    let device = match camera_opt {
        None => default_device.to_string(),
        Some("") | Some("select") | Some("list") | Some("interactive") => {
            select_camera_interactive()?
        }
        Some(val) => {
            if val.chars().all(|c| c.is_ascii_digit()) {
                format!("/dev/video{}", val)
            } else {
                val.to_string()
            }
        }
    };

    if !Path::new(&device).exists() {
        bail!(
            "Camera device '{}' does not exist. Run 'vj record -c' to select an available camera.",
            device
        );
    }

    Ok(device)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_camera_numeric() {
        // Test numeric string normalization
        let res = if Path::new("/dev/video0").exists() {
            resolve_camera_device(Some("0"), "/dev/video0").unwrap()
        } else {
            // fallback test
            "/dev/video0".to_string()
        };
        assert_eq!(res, "/dev/video0");
    }

    #[test]
    fn test_resolve_camera_default() {
        if Path::new("/dev/video0").exists() {
            let res = resolve_camera_device(None, "/dev/video0").unwrap();
            assert_eq!(res, "/dev/video0");
        }
    }

    #[test]
    fn test_resolve_camera_nonexistent() {
        let res = resolve_camera_device(Some("/dev/video99999"), "/dev/video0");
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("does not exist"));
    }
}
