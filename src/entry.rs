use crate::calendar::CalendarSystem;
use crate::crypto::{self, GpgAuth};
use anyhow::{Context, Result};
use colored::*;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub timestamp: String,
    #[serde(default)]
    pub calendar: Option<String>,
    pub title: String,
    #[serde(default)]
    pub original_filename: Option<String>,
    #[serde(default)]
    pub imported: Option<bool>,
    pub profile: String,
    #[serde(default)]
    pub resolution: Option<String>,
    #[serde(default)]
    pub fps: Option<u32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub encrypted: bool,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub id: String,
    pub dir: PathBuf,
    pub is_encrypted: bool,
    pub is_encoding: bool,
    #[allow(dead_code)]
    pub video_path: Option<PathBuf>,
    pub size_bytes: u64,
    pub meta: Option<Meta>,
}

impl Entry {
    pub fn from_dir<P: AsRef<Path>>(dir: P, temp_dir: &Path) -> Option<Self> {
        let dir = dir.as_ref().to_path_buf();
        if !dir.is_dir() {
            return None;
        }

        let id = dir.file_name()?.to_string_lossy().to_string();

        let v_plain = dir.join("video.mkv");
        let v_gpg = dir.join("video.mkv.gpg");
        let raw_temp = temp_dir.join(format!("raw_{}.mkv", id));

        let is_encrypted = v_gpg.exists();
        let is_encoding = raw_temp.exists() && !v_plain.exists() && !v_gpg.exists();

        let (video_path, size_bytes) = if is_encrypted {
            let sz = fs::metadata(&v_gpg).map(|m| m.len()).unwrap_or(0);
            (Some(v_gpg), sz)
        } else if v_plain.exists() {
            let sz = fs::metadata(&v_plain).map(|m| m.len()).unwrap_or(0);
            (Some(v_plain), sz)
        } else {
            (None, 0)
        };

        // Try reading meta.json if not encrypted
        let meta_file = dir.join("meta.json");
        let meta = if meta_file.exists() {
            fs::read_to_string(&meta_file)
                .ok()
                .and_then(|s| serde_json::from_str::<Meta>(&s).ok())
        } else {
            None
        };

        Some(Self {
            id,
            dir,
            is_encrypted,
            is_encoding,
            video_path,
            size_bytes,
            meta,
        })
    }

    pub fn formatted_size(&self) -> String {
        if self.is_encoding {
            return "(enc...)".to_string();
        }
        let bytes = self.size_bytes;
        if bytes >= 1024 * 1024 * 1024 {
            format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
        } else if bytes >= 1024 * 1024 {
            format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
        } else if bytes >= 1024 {
            format!("{:.1}K", bytes as f64 / 1024.0)
        } else {
            format!("{}B", bytes)
        }
    }

    pub fn title(&self) -> String {
        self.meta
            .as_ref()
            .map(|m| m.title.clone())
            .unwrap_or_else(|| format!("Entry {}", self.id))
    }

    pub fn tags_string(&self) -> String {
        if let Some(ref m) = self.meta {
            if !m.tags.is_empty() {
                return format!("[\"{}\"]", m.tags.join("\", \""));
            }
        }
        "[]".to_string()
    }
}

pub fn load_entries(entries_dir: &Path, temp_dir: &Path) -> Result<Vec<Entry>> {
    migrate_legacy_files(entries_dir)?;

    if !entries_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in fs::read_dir(entries_dir).context("Failed to read entries directory")? {
        let entry = entry?;
        let p = entry.path();
        if p.is_dir() {
            if let Some(e) = Entry::from_dir(p, temp_dir) {
                entries.push(e);
            }
        }
    }

    // Sort reverse-chronological (newest first)
    entries.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(entries)
}

pub fn find_entry<'a>(entries: &'a [Entry], target: &str) -> Option<&'a Entry> {
    // 1. Exact match
    if let Some(e) = entries.iter().find(|e| e.id == target) {
        return Some(e);
    }
    // 2. Partial match
    entries.iter().find(|e| e.id.contains(target))
}

pub fn migrate_legacy_files(entries_dir: &Path) -> Result<()> {
    let parent = entries_dir.parent().unwrap_or(entries_dir);
    if let Ok(rd) = fs::read_dir(parent) {
        for item in rd.flatten() {
            let path = item.path();
            if let Some(fname) = path.file_name().and_then(|n| n.to_str()) {
                if (fname.starts_with("entry_") && fname.ends_with(".mkv"))
                    || (fname.starts_with("entry_") && fname.ends_with(".mkv.gpg"))
                {
                    let clean = fname.trim_start_matches("entry_");
                    let stamp = clean.split('.').next().unwrap_or(clean);
                    if !stamp.is_empty() {
                        let entry_folder = entries_dir.join(stamp);
                        let _ = fs::create_dir_all(&entry_folder);
                        let is_gpg = fname.ends_with(".gpg");
                        let dest_name = if is_gpg { "video.mkv.gpg" } else { "video.mkv" };
                        let _ = fs::rename(&path, entry_folder.join(dest_name));

                        let meta_path = entry_folder.join("meta.json");
                        let meta_gpg = entry_folder.join("meta.json.gpg");
                        if !meta_path.exists() && !meta_gpg.exists() {
                            let meta_json = format!(
                                r#"{{
  "timestamp": "{}",
  "profile": "legacy",
  "tags": ["migrated"],
  "title": "Migrated Entry {}"
}}"#,
                                stamp, stamp
                            );
                            let _ = fs::write(meta_path, meta_json);
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn print_list(entries: &[Entry], quiet: bool) {
    if entries.is_empty() {
        if !quiet {
            println!("No entries found.");
        }
        return;
    }

    if quiet {
        for e in entries {
            println!("{}", e.id);
        }
        return;
    }

    println!(
        "{:<22} {:<10} {:<8} {:<32} {}",
        "ID / TIMESTAMP", "STATUS", "SIZE", "TITLE", "TAGS"
    );
    println!(
        "{:<22} {:<10} {:<8} {:<32} {}",
        "----------------------", "----------", "--------", "--------------------------------", "----------------"
    );

    for e in entries {
        let status = if e.is_encrypted {
            "[GPG]".yellow()
        } else {
            "[RAW]".green()
        };

        println!(
            "{:<22} {:<10} {:<8} {:<32} {}",
            e.id,
            status,
            e.formatted_size(),
            e.title(),
            e.tags_string()
        );
    }
}

pub fn print_preview(entry: &Entry, auth: &GpgAuth) -> Result<()> {
    println!("=================== ENTRY DETAILS ===================");
    println!("Timestamp:    {}", entry.id);
    println!("Location:     {}", entry.dir.display());

    let v_gpg = entry.dir.join("video.mkv.gpg");
    let v_plain = entry.dir.join("video.mkv");
    let note_plain = entry.dir.join("note.md");
    let note_gpg = entry.dir.join("note.md.gpg");
    let meta_plain = entry.dir.join("meta.json");
    let meta_gpg = entry.dir.join("meta.json.gpg");

    if v_gpg.exists() {
        println!("Status:       🔒 AES-256 GPG Encrypted");
        println!("Size:         {}", entry.formatted_size());

        if meta_gpg.exists() {
            println!("\n--- Metadata ---");
            if auth.has_auth() {
                if let Ok(dec) = crypto::decrypt_to_string(&meta_gpg, auth) {
                    println!("{}", dec.trim());
                } else {
                    println!("(Encrypted metadata)");
                }
            } else {
                println!("(Metadata is GPG encrypted)");
            }
        }

        if note_gpg.exists() {
            println!("\n--- Note Preview ---");
            if auth.has_auth() {
                if let Ok(dec) = crypto::decrypt_to_string(&note_gpg, auth) {
                    let lines: Vec<&str> = dec.lines().take(25).collect();
                    println!("{}", lines.join("\n"));
                } else {
                    println!("(Encrypted note)");
                }
            } else {
                println!("(Note is GPG encrypted)");
            }
        }
    } else if v_plain.exists() {
        println!("Status:       Plaintext");
        println!("Size:         {}", entry.formatted_size());

        if meta_plain.exists() {
            println!("\n--- Metadata ---");
            if let Ok(txt) = fs::read_to_string(&meta_plain) {
                println!("{}", txt.trim());
            }
        }

        if note_plain.exists() {
            println!("\n--- Note Preview ---");
            if let Ok(txt) = fs::read_to_string(&note_plain) {
                let lines: Vec<&str> = txt.lines().take(25).collect();
                println!("{}", lines.join("\n"));
            }
        }
    } else if entry.is_encoding {
        println!("Status:       Compressing in background...");
    }

    Ok(())
}

pub fn print_stats(entries: &[Entry], entries_dir: &Path, inbox_dir: &Path, cal: CalendarSystem) {
    let total_entries = entries.len();
    let mut encrypted_count = 0;
    let mut raw_count = 0;
    let mut total_bytes: u64 = 0;
    let mut days_recorded = std::collections::HashSet::new();

    for e in entries {
        if e.is_encrypted {
            encrypted_count += 1;
        } else {
            raw_count += 1;
        }
        total_bytes += e.size_bytes;

        let day = e.id.split('_').next().unwrap_or(&e.id);
        days_recorded.insert(day.to_string());
    }

    let disk_usage = if total_bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", total_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if total_bytes >= 1024 * 1024 {
        format!("{:.1} MB", total_bytes as f64 / (1024.0 * 1024.0))
    } else if total_bytes >= 1024 {
        format!("{:.1} KB", total_bytes as f64 / 1024.0)
    } else {
        format!("{} B", total_bytes)
    };

    println!("Location:       {}", entries_dir.display());
    println!("Inbox:          {}", inbox_dir.display());
    println!("Calendar:       {}", cal);
    println!("Total Entries:  {}", total_entries);
    println!("Plaintext:      {}", raw_count);
    println!("Encrypted:      {}", encrypted_count);
    println!("Storage:        {}", disk_usage);
    println!("Recorded Days:  {}", days_recorded.len());
}

pub fn delete_entry(entry: &Entry, force: bool) -> Result<()> {
    if !force {
        let prompt = format!(
            "Are you sure you want to permanently delete entry '{}' ({})?",
            entry.id, entry.title()
        );
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Deletion cancelled.");
            return Ok(());
        }
    }

    fs::remove_dir_all(&entry.dir)
        .with_context(|| format!("Failed to delete entry directory {:?}", entry.dir))?;

    println!("[✓] Deleted entry '{}'", entry.id);
    Ok(())
}
