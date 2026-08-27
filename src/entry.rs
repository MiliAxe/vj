use crate::calendar::{self, CalendarSystem};
use crate::crypto::{self, GpgAuth};
use anyhow::{Context, Result};
use chrono::{Datelike, Duration, Local, NaiveDate};
use colored::*;
use dialoguer::Confirm;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

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

    render_entry_thumbnail(entry, auth);

    Ok(())
}

fn render_entry_thumbnail(entry: &Entry, auth: &GpgAuth) {
    let thumb_file = entry.dir.join("thumb.jpg");
    let thumb_gpg = entry.dir.join("thumb.jpg.gpg");
    let v_plain = entry.dir.join("video.mkv");

    // 1. If plaintext thumb doesn't exist, try to generate it from video.mkv if present
    if !thumb_file.exists() && !thumb_gpg.exists() && v_plain.exists() {
        let _ = Command::new("ffmpeg")
            .arg("-loglevel").arg("error")
            .arg("-y")
            .arg("-i").arg(&v_plain)
            .arg("-vf").arg("thumbnail=20,scale=160:120,tile=2x2")
            .arg("-frames:v").arg("1")
            .arg(&thumb_file)
            .status();
    }

    let mut temp_thumb_path: Option<PathBuf> = None;

    let target_thumb: Option<PathBuf> = if thumb_file.exists() {
        Some(thumb_file)
    } else if thumb_gpg.exists() && auth.has_auth() {
        let tmp = std::env::temp_dir().join(format!("vj_thumb_prev_{}.jpg", entry.id));
        if crypto::decrypt_file(&thumb_gpg, &tmp, auth).is_ok() {
            temp_thumb_path = Some(tmp.clone());
            Some(tmp)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(ref path) = target_thumb {
        if let Ok(chafa) = which::which("chafa") {
            println!("\n{}", "--- Storyboard Preview ---".dimmed());
            let _ = Command::new(chafa)
                .arg("--size=38x13")
                .arg(path)
                .status();
        } else if let Ok(timg) = which::which("timg") {
            println!("\n{}", "--- Storyboard Preview ---".dimmed());
            let _ = Command::new(timg)
                .arg("-g38x13")
                .arg(path)
                .status();
        } else if let Ok(viu) = which::which("viu") {
            println!("\n{}", "--- Storyboard Preview ---".dimmed());
            let _ = Command::new(viu)
                .arg("-w").arg("38")
                .arg(path)
                .status();
        }
    }

    if let Some(tmp) = temp_thumb_path {
        let _ = fs::remove_file(tmp);
    }
}

pub fn print_stats(
    entries: &[Entry],
    entries_dir: &Path,
    inbox_dir: &Path,
    cal: CalendarSystem,
    months: u32,
) {
    let total_entries = entries.len();
    let mut encrypted_count = 0;
    let mut raw_count = 0;
    let mut total_bytes: u64 = 0;

    let mut date_counts: HashMap<NaiveDate, usize> = HashMap::new();
    let mut all_dates_set: BTreeSet<NaiveDate> = BTreeSet::new();

    for e in entries {
        if e.is_encrypted {
            encrypted_count += 1;
        } else {
            raw_count += 1;
        }
        total_bytes += e.size_bytes;

        let declared_cal = e.meta.as_ref().and_then(|m| m.calendar.as_deref());
        if let Some(nd) = calendar::parse_entry_to_naive_date(&e.id, declared_cal) {
            *date_counts.entry(nd).or_insert(0) += 1;
            all_dates_set.insert(nd);
        }
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

    println!("================== JOURNAL STATS ==================");
    println!("Location:       {}", entries_dir.display());
    println!("Inbox:          {}", inbox_dir.display());
    println!("Calendar:       {}", cal);
    println!("Total Entries:  {}", total_entries);
    println!("Plaintext:      {}", raw_count);
    println!("Encrypted:      {}", encrypted_count);
    println!("Storage:        {}", disk_usage);
    println!("Recorded Days:  {}", all_dates_set.len());

    if entries.is_empty() {
        return;
    }

    // 1. Calculate Streaks
    let today = Local::now().date_naive();
    let yesterday = today - Duration::days(1);

    let mut current_streak = 0;
    let mut streak_start_date = None;

    let streak_anchor = if date_counts.contains_key(&today) {
        Some(today)
    } else if date_counts.contains_key(&yesterday) {
        Some(yesterday)
    } else {
        None
    };

    if let Some(mut curr) = streak_anchor {
        while date_counts.contains_key(&curr) {
            current_streak += 1;
            streak_start_date = Some(curr);
            curr = curr - Duration::days(1);
        }
    }

    let mut longest_streak = 0;
    let mut temp_streak = 0;
    let mut prev_date: Option<NaiveDate> = None;

    for &d in &all_dates_set {
        if let Some(prev) = prev_date {
            if d == prev + Duration::days(1) {
                temp_streak += 1;
            } else {
                temp_streak = 1;
            }
        } else {
            temp_streak = 1;
        }
        if temp_streak > longest_streak {
            longest_streak = temp_streak;
        }
        prev_date = Some(d);
    }

    let effective_months = months.max(1);
    let total_days_window = (effective_months * 30) as i64;
    let window_start = today - Duration::days(total_days_window - 1);
    let active_in_window = all_dates_set.iter().filter(|&&d| d >= window_start && d <= today).count();
    let active_pct = (active_in_window as f64 / total_days_window as f64 * 100.0).round() as u32;

    println!("\nStreaks:");
    if current_streak > 0 {
        let start_str = if let Some(st) = streak_start_date {
            format_date_for_display(st, cal)
        } else {
            "".to_string()
        };
        println!(
            "  :: Current Streak: {} day{} ({} -> Today)",
            current_streak,
            if current_streak == 1 { "" } else { "s" },
            start_str
        );
    } else {
        println!("  :: Current Streak: 0 days");
    }
    println!(
        "  :: Longest Streak: {} day{}",
        longest_streak,
        if longest_streak == 1 { "" } else { "s" }
    );
    println!(
        "  :: Active Days:    {} / {} days ({}%)",
        active_in_window, total_days_window, active_pct
    );

    // 2. Render Retro CRT Contribution Heatmap
    render_contribution_heatmap(&date_counts, today, cal, effective_months);
}

fn format_date_for_display(d: NaiveDate, cal: CalendarSystem) -> String {
    match cal {
        CalendarSystem::Jalali => {
            let (jy, jm, jd) = calendar::gregorian_to_jalali(d.year(), d.month(), d.day());
            format!("{:04}-{:02}-{:02}", jy, jm, jd)
        }
        CalendarSystem::Gregorian => d.format("%Y-%m-%d").to_string(),
    }
}

fn day_of_week_index(date: NaiveDate, cal: CalendarSystem) -> usize {
    match cal {
        CalendarSystem::Jalali => {
            // Saturday is index 0
            ((date.weekday().num_days_from_monday() + 2) % 7) as usize
        }
        CalendarSystem::Gregorian => {
            // Monday is index 0
            date.weekday().num_days_from_monday() as usize
        }
    }
}

fn month_abbrev(date: NaiveDate, cal: CalendarSystem) -> &'static str {
    match cal {
        CalendarSystem::Jalali => {
            let (_, jm, _) = calendar::gregorian_to_jalali(date.year(), date.month(), date.day());
            match jm {
                1 => "Far",
                2 => "Ord",
                3 => "Kho",
                4 => "Tir",
                5 => "Mor",
                6 => "Sha",
                7 => "Meh",
                8 => "Abn",
                9 => "Aza",
                10 => "Dey",
                11 => "Bah",
                12 => "Esf",
                _ => "---",
            }
        }
        CalendarSystem::Gregorian => match date.month() {
            1 => "Jan",
            2 => "Feb",
            3 => "Mar",
            4 => "Apr",
            5 => "May",
            6 => "Jun",
            7 => "Jul",
            8 => "Aug",
            9 => "Sep",
            10 => "Oct",
            11 => "Nov",
            12 => "Dec",
            _ => "---",
        },
    }
}

fn month_key(date: NaiveDate, cal: CalendarSystem) -> (i32, u32) {
    match cal {
        CalendarSystem::Jalali => {
            let (jy, jm, _) = calendar::gregorian_to_jalali(date.year(), date.month(), date.day());
            (jy, jm)
        }
        CalendarSystem::Gregorian => (date.year(), date.month()),
    }
}

fn render_contribution_heatmap(
    date_counts: &HashMap<NaiveDate, usize>,
    today: NaiveDate,
    cal: CalendarSystem,
    months: u32,
) {
    let day_labels: [&str; 7] = match cal {
        CalendarSystem::Jalali => ["Sat", "Sun", "Mon", "Tue", "Wed", "Thu", "Fri"],
        CalendarSystem::Gregorian => ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"],
    };

    let total_weeks = if months == 12 {
        52
    } else {
        (months * 4 + (months / 3).max(1)) as usize
    };

    let days_to_end_of_week = 6 - day_of_week_index(today, cal);
    let grid_end_date = today + Duration::days(days_to_end_of_week as i64);
    let total_days = (total_weeks * 7) as i64;
    let grid_start_date = grid_end_date - Duration::days(total_days - 1);

    println!(
        "\nActivity (Past {} Month{}):",
        months,
        if months == 1 { "" } else { "s" }
    );

    // Month headers row
    let mut header_chars: Vec<char> = vec![' '; 5 + total_weeks * 2 + 4];
    let mut prev_month_key = None;

    for w in 0..total_weeks {
        let mid_week_date = grid_start_date + Duration::days((w * 7 + 3) as i64);
        let m_key = month_key(mid_week_date, cal);
        if prev_month_key != Some(m_key) {
            let name = month_abbrev(mid_week_date, cal);
            let pos = 5 + w * 2;
            for (idx, ch) in name.chars().enumerate() {
                if pos + idx < header_chars.len() {
                    header_chars[pos + idx] = ch;
                }
            }
            prev_month_key = Some(m_key);
        }
    }

    let header_str: String = header_chars.into_iter().collect();
    println!("{}", header_str.trim_end().dimmed());

    // 7 rows for each day of the week
    for r in 0..7 {
        let mut row_str = format!("{:<4} ", day_labels[r]);
        for w in 0..total_weeks {
            let day_date = grid_start_date + Duration::days((w * 7 + r) as i64);
            if day_date > today {
                row_str.push_str("  ");
            } else {
                let count = date_counts.get(&day_date).copied().unwrap_or(0);
                let cell = match count {
                    0 => "░ ".dimmed().to_string(),
                    1 => "▒ ".green().to_string(),
                    2 => "▓ ".bright_green().to_string(),
                    _ => "█ ".bold().bright_cyan().to_string(),
                };
                row_str.push_str(&cell);
            }
        }
        println!("{}", row_str);
    }

    println!(
        "\nLegend:  {} 0  {} 1  {} 2  {} 3+",
        "░".dimmed(),
        "▒".green(),
        "▓".bright_green(),
        "█".bold().bright_cyan()
    );
}


pub fn execute_delete(
    entry_ids: Vec<String>,
    force: bool,
    entries_path: &Path,
    temp_path: &Path,
) -> Result<()> {
    let entries = load_entries(entries_path, temp_path)?;
    if entries.is_empty() {
        println!("No entries found in {}", entries_path.display());
        return Ok(());
    }

    let mut target_ids = entry_ids;

    // If no target IDs given, run interactive fzf multi-select browser
    if target_ids.is_empty() {
        if std::io::stdin().is_terminal() && which::which("fzf").is_ok() {
            let current_exe = std::env::current_exe()?;
            let preview_cmd = format!("{} preview {{1}}", current_exe.display());
            let peek_cmd = format!("{} __peek {{1}}", current_exe.display());

            let mut fzf_cmd = std::process::Command::new("fzf");
            fzf_cmd
                .arg("-m")
                .arg("--prompt=vj delete (TAB to multi-select) > ")
                .arg("--header=[TAB: Multi-select | Enter: Confirm | Ctrl-P / Space: Peek Video | Esc: Cancel]")
                .arg(format!("--preview={}", preview_cmd))
                .arg("--preview-window=right:55%:wrap")
                .arg(format!("--bind=ctrl-p:execute-silent({}),space:execute-silent({})", peek_cmd, peek_cmd))
                .arg("--bind=ctrl-j:down,ctrl-k:up,ctrl-d:page-down,ctrl-u:page-up,ctrl-y:preview-up,ctrl-e:preview-down")
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped());

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
                let stdout_str = String::from_utf8_lossy(&output.stdout);
                for line in stdout_str.lines() {
                    if let Some(first_word) = line.split_whitespace().next() {
                        target_ids.push(first_word.to_string());
                    }
                }
            } else {
                println!("Deletion cancelled.");
                return Ok(());
            }
        } else {
            anyhow::bail!("Please specify at least one entry ID to delete (e.g. vj delete <id1> <id2>)");
        }
    }

    if target_ids.is_empty() {
        println!("No entries selected.");
        return Ok(());
    }

    // Resolve matching entries
    let mut to_delete: Vec<&Entry> = Vec::new();
    for target in &target_ids {
        if let Some(entry) = find_entry(&entries, target) {
            if !to_delete.iter().any(|e| e.id == entry.id) {
                to_delete.push(entry);
            }
        } else {
            eprintln!("Warning: Entry '{}' not found, skipping.", target);
        }
    }

    if to_delete.is_empty() {
        println!("No valid entries found to delete.");
        return Ok(());
    }

    // Confirmation prompt (unless force is provided)
    if !force {
        println!("The following {} entry/entries will be permanently deleted:", to_delete.len());
        for e in &to_delete {
            println!("  - {}  {}", e.id.bold(), e.title().dimmed());
        }
        let prompt = if to_delete.len() == 1 {
            format!("Are you sure you want to permanently delete '{}'?", to_delete[0].id)
        } else {
            format!("Are you sure you want to permanently delete these {} entries?", to_delete.len())
        };
        let confirmed = Confirm::new()
            .with_prompt(prompt)
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Deletion cancelled.");
            return Ok(());
        }
    }

    // Delete directories
    for entry in to_delete {
        fs::remove_dir_all(&entry.dir)
            .with_context(|| format!("Failed to delete entry directory {:?}", entry.dir))?;
        println!("[✓] Deleted entry '{}'", entry.id);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_delete_multiple_entries() {
        let base_temp = std::env::temp_dir().join(format!("vj_test_del_{}", std::process::id()));
        let entries_dir = base_temp.join("entries");
        let temp_dir = base_temp.join("temp");
        let _ = fs::remove_dir_all(&base_temp);
        fs::create_dir_all(&entries_dir).unwrap();
        fs::create_dir_all(&temp_dir).unwrap();

        let e1_dir = entries_dir.join("1405-01-01_10-00-00");
        let e2_dir = entries_dir.join("1405-01-02_10-00-00");
        let e3_dir = entries_dir.join("1405-01-03_10-00-00");
        fs::create_dir_all(&e1_dir).unwrap();
        fs::create_dir_all(&e2_dir).unwrap();
        fs::create_dir_all(&e3_dir).unwrap();

        fs::write(e1_dir.join("video.mkv"), b"fake").unwrap();
        fs::write(e2_dir.join("video.mkv"), b"fake").unwrap();
        fs::write(e3_dir.join("video.mkv"), b"fake").unwrap();

        // Delete e1 and e2 with force=true
        execute_delete(
            vec!["1405-01-01_10-00-00".to_string(), "1405-01-02_10-00-00".to_string()],
            true,
            &entries_dir,
            &temp_dir,
        )
        .unwrap();

        assert!(!e1_dir.exists());
        assert!(!e2_dir.exists());
        assert!(e3_dir.exists());

        let _ = fs::remove_dir_all(&base_temp);
    }
}
