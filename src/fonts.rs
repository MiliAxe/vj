use crate::config::{expand_path, get_config_dir};
use anyhow::Result;
use colored::*;
use std::fs;
use std::path::{Path, PathBuf};

// Embedded retro fonts (zero runtime network dependency)
pub const FONT_VT323: &[u8] = include_bytes!("../assets/vt323.ttf");
pub const FONT_SILKSCREEN: &[u8] = include_bytes!("../assets/silkscreen.ttf");
pub const FONT_PRESS_START_2P: &[u8] = include_bytes!("../assets/press_start_2p.ttf");
pub const FONT_SHARE_TECH_MONO: &[u8] = include_bytes!("../assets/share_tech_mono.ttf");

pub fn get_fonts_dir() -> PathBuf {
    get_config_dir().join("fonts")
}

/// Ensure embedded retro fonts are written to ~/.config/vj/fonts/
pub fn ensure_embedded_fonts() -> Result<()> {
    let fonts_dir = get_fonts_dir();
    fs::create_dir_all(&fonts_dir)?;

    let write_if_missing = |name: &str, data: &[u8]| {
        let p = fonts_dir.join(name);
        if !p.exists() {
            let _ = fs::write(p, data);
        }
    };

    write_if_missing("vt323.ttf", FONT_VT323);
    write_if_missing("silkscreen.ttf", FONT_SILKSCREEN);
    write_if_missing("press_start_2p.ttf", FONT_PRESS_START_2P);
    write_if_missing("share_tech_mono.ttf", FONT_SHARE_TECH_MONO);

    Ok(())
}

/// Resolve a font specification (name, relative path, absolute path, or system font name)
/// Returns a drawtext font parameter: `fontfile='...'` or `font='...'`
pub fn resolve_font_param(font_name: &str) -> String {
    let _ = ensure_embedded_fonts();
    let fonts_dir = get_fonts_dir();

    let clean = font_name.trim().to_lowercase().replace('-', "_");

    // 1. Check embedded / config fonts directory
    let mapped_file = match clean.as_str() {
        "vt323" | "vcr" | "crt" | "retro" | "default" => Some("vt323.ttf"),
        "silkscreen" | "pixel" | "camcorder" | "8bit" => Some("silkscreen.ttf"),
        "press_start" | "press_start_2p" | "arcade" => Some("press_start_2p.ttf"),
        "share_tech" | "share_tech_mono" | "tech" | "hud" => Some("share_tech_mono.ttf"),
        _ => None,
    };

    if let Some(fname) = mapped_file {
        let p = fonts_dir.join(fname);
        if p.exists() {
            return format!("fontfile='{}'", escape_path_for_ffmpeg(&p));
        }
    }

    // 2. Check if font_name is a direct file path (e.g. ~/myfont.ttf or /path/to/font.ttf)
    let direct_path = expand_path(font_name);
    if direct_path.exists() && direct_path.is_file() {
        return format!("fontfile='{}'", escape_path_for_ffmpeg(&direct_path));
    }

    // 3. Check if file exists inside fonts directory with custom name
    let config_custom = fonts_dir.join(font_name);
    if config_custom.exists() && config_custom.is_file() {
        return format!("fontfile='{}'", escape_path_for_ffmpeg(&config_custom));
    }

    // 4. Default fallback: use embedded vt323 if available
    let default_vt323 = fonts_dir.join("vt323.ttf");
    if default_vt323.exists() {
        return format!("fontfile='{}'", escape_path_for_ffmpeg(&default_vt323));
    }

    // 5. System font name fallback (e.g. font='Monospace')
    format!("font='{}'", font_name)
}

fn escape_path_for_ffmpeg(p: &Path) -> String {
    p.to_string_lossy()
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
}

pub fn print_recommended_fonts() {
    let _ = ensure_embedded_fonts();
    let fonts_dir = get_fonts_dir();

    println!("{}", "vj Recommended Retro Fonts:".bold());
    println!();
    println!(
        "{:<20} {:<24} {}",
        "FONT IDENTIFIER", "STYLE / ERA", "DESCRIPTION"
    );
    println!(
        "{:<20} {:<24} {}",
        "-------------------", "-----------------------", "----------------------------------------------"
    );

    println!(
        "{:<20} {:<24} {}",
        "vt323 (*)".green(),
        "DEC VT323 CRT / VHS",
        "Iconic tall retro VHS & CRT phosphor terminal font"
    );
    println!(
        "{:<20} {:<24} {}",
        "silkscreen".yellow(),
        "90s Handheld Camcorder",
        "Ultra-crisp pixel matrix font, ideal for compact/potato"
    );
    println!(
        "{:<20} {:<24} {}",
        "press_start_2p".cyan(),
        "8-Bit Arcade / Micro",
        "Classic 1980s retro gaming & computer pixel typography"
    );
    println!(
        "{:<20} {:<24} {}",
        "share_tech_mono".magenta(),
        "Cyberpunk HUD / Sci-Fi",
        "Modern high-tech vintage monospace HUD display font"
    );

    println!();
    println!("Fonts Directory: {}", fonts_dir.display());
    println!();
    println!("Usage:");
    println!("  vj record --overlay-font vt323");
    println!("  vj record --overlay-font silkscreen --overlay-style green");
    println!("  vj record --overlay-font /path/to/custom_font.ttf");
    println!();
    println!("Set default font in ~/.config/vj/config.toml via overlay_font = \"vt323\"");
}
