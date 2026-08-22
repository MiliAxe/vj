use crate::calendar::CalendarSystem;
use crate::profile::Profile;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_entries_dir")]
    pub journal_dir: String,

    #[serde(default = "default_inbox_dir")]
    pub inbox_dir: String,

    #[serde(default = "default_temp_dir")]
    pub temp_dir: String,

    #[serde(default = "default_calendar")]
    pub date_calendar: String,

    #[serde(default = "default_profile")]
    pub default_profile: String,

    #[serde(default = "default_camera_dev")]
    pub camera_dev: String,

    #[serde(default = "default_audio_src")]
    pub audio_src: String,

    #[serde(default = "default_editor")]
    pub editor: String,

    #[serde(default = "default_inbox_port")]
    pub inbox_port: u16,

    #[serde(default)]
    pub auto_encrypt: bool,

    pub key_file: Option<String>,

    pub passphrase: Option<String>,

    pub extra_ffmpeg_flags: Option<String>,

    #[serde(default)]
    pub profiles: HashMap<String, Profile>,
}

fn default_entries_dir() -> String {
    "~/Videos/Journal/entries".to_string()
}

fn default_inbox_dir() -> String {
    "~/Videos/Journal/inbox".to_string()
}

fn default_temp_dir() -> String {
    "/tmp/vj_temp".to_string()
}

fn default_calendar() -> String {
    "jalali".to_string()
}

fn default_profile() -> String {
    "terry".to_string()
}

fn default_camera_dev() -> String {
    "/dev/video0".to_string()
}

fn default_audio_src() -> String {
    "default".to_string()
}

fn default_editor() -> String {
    if which::which("nvim").is_ok() {
        "nvim".to_string()
    } else {
        "vim".to_string()
    }
}

fn default_inbox_port() -> u16 {
    8080
}

impl Default for Config {
    fn default() -> Self {
        Self {
            journal_dir: default_entries_dir(),
            inbox_dir: default_inbox_dir(),
            temp_dir: default_temp_dir(),
            date_calendar: default_calendar(),
            default_profile: default_profile(),
            camera_dev: default_camera_dev(),
            audio_src: default_audio_src(),
            editor: default_editor(),
            inbox_port: default_inbox_port(),
            auto_encrypt: false,
            key_file: None,
            passphrase: None,
            extra_ffmpeg_flags: None,
            profiles: HashMap::new(),
        }
    }
}

pub fn expand_path(p: &str) -> PathBuf {
    if p.starts_with("~/") || p == "~" {
        if let Some(home) = dirs_home() {
            if p == "~" {
                return home;
            } else {
                return home.join(&p[2..]);
            }
        }
    }
    PathBuf::from(p)
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

pub fn get_config_dir() -> PathBuf {
    if let Ok(val) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(val).join("vj")
    } else if let Some(home) = dirs_home() {
        home.join(".config").join("vj")
    } else {
        PathBuf::from("./config")
    }
}

pub fn get_config_file() -> PathBuf {
    get_config_dir().join("config.toml")
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_file = get_config_file();
        let config_dir = get_config_dir();

        // Check if legacy config.env exists
        let legacy_env = config_dir.join("config.env");
        if !config_file.exists() && legacy_env.exists() {
            eprintln!(
                "Note: Found legacy config at {:?}. Creating TOML configuration at {:?}",
                legacy_env, config_file
            );
        }

        if !config_file.exists() {
            fs::create_dir_all(&config_dir).context("Failed to create config directory")?;
            let default_cfg = Config::default();
            default_cfg.save(&config_file)?;
            return Ok(default_cfg);
        }

        let content = fs::read_to_string(&config_file)
            .with_context(|| format!("Failed to read config file {:?}", config_file))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML in {:?}", config_file))?;
        Ok(config)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let toml_str = toml::to_string_pretty(self).context("Failed to serialize config to TOML")?;
        fs::write(path, toml_str).context("Failed to write config file")?;
        Ok(())
    }

    pub fn entries_path(&self) -> PathBuf {
        expand_path(&self.journal_dir)
    }

    pub fn inbox_path(&self) -> PathBuf {
        expand_path(&self.inbox_dir)
    }

    pub fn temp_path(&self) -> PathBuf {
        expand_path(&self.temp_dir)
    }

    pub fn key_file_path(&self) -> Option<PathBuf> {
        self.key_file.as_ref().map(|s| expand_path(s))
    }

    pub fn calendar_system(&self) -> CalendarSystem {
        self.date_calendar.parse().unwrap_or_default()
    }

    pub fn ensure_directories(&self) -> Result<()> {
        fs::create_dir_all(self.entries_path()).context("Failed to create entries directory")?;
        fs::create_dir_all(self.inbox_path()).context("Failed to create inbox directory")?;
        fs::create_dir_all(self.temp_path()).context("Failed to create temp directory")?;
        Ok(())
    }
}
