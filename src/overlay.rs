use crate::fonts;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OverlayStyle {
    #[default]
    VhsYellow,
    CamcorderWhite,
    Green,
    Amber,
    Cyan,
}

impl std::str::FromStr for OverlayStyle {
    type Err = std::convert::Infallible;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().replace('-', "_").as_str() {
            "white" | "camcorder_white" | "camcorder" => Ok(OverlayStyle::CamcorderWhite),
            "green" | "phosphor" | "crt" => Ok(OverlayStyle::Green),
            "amber" | "orange" => Ok(OverlayStyle::Amber),
            "cyan" => Ok(OverlayStyle::Cyan),
            _ => Ok(OverlayStyle::VhsYellow),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub style: OverlayStyle,
    pub font: String,
    pub font_size: Option<u32>,
    pub show_title: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            style: OverlayStyle::VhsYellow,
            font: "vt323".to_string(),
            font_size: None,
            show_title: true,
        }
    }
}

fn escape_drawtext_str(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace('%', "\\%")
}

/// Build the FFmpeg filterchain string for retro OSD
pub fn build_drawtext_filter(
    timestamp_raw: &str,
    title_opt: Option<&str>,
    cfg: &OverlayConfig,
    resolution: &str,
) -> Option<String> {
    if !cfg.enabled {
        return None;
    }

    let height: u32 = resolution
        .split('x')
        .nth(1)
        .and_then(|h| h.parse().ok())
        .unwrap_or(480);

    let fontsize = cfg
        .font_size
        .filter(|&s| s > 0)
        .unwrap_or_else(|| (height / 24).max(14));

    let margin = (height / 22).max(14);
    let stroke_width = (fontsize / 12).max(1);

    let font_param = fonts::resolve_font_param(&cfg.font);

    let fontcolor = match cfg.style {
        OverlayStyle::VhsYellow => "0xFFE500",
        OverlayStyle::CamcorderWhite => "0xF0F0F0",
        OverlayStyle::Green => "0x33FF33",
        OverlayStyle::Amber => "0xFFB000",
        OverlayStyle::Cyan => "0x00E5FF",
    };

    let stroke_opts = format!("borderw={}:bordercolor=black@0.95", stroke_width);

    // Format raw timestamp e.g. "1405-05-30_12-33-03" -> "1405-05-30  12:33:03"
    let formatted_ts = if let Some((d, t)) = timestamp_raw.split_once('_') {
        let clean_t = t.replace('-', ":");
        format!("{}  {}", d, clean_t)
    } else {
        timestamp_raw.to_string()
    };

    let escaped_ts = escape_drawtext_str(&formatted_ts);
    let mut filters = Vec::new();

    let title_font_size = (fontsize * 88) / 100;
    let has_custom_title = cfg.show_title
        && title_opt
            .map(|t| !t.starts_with("Entry ") && !t.trim().is_empty())
            .unwrap_or(false);

    let line_spacing = (fontsize * 125) / 100;

    // 1. Title Filter (stacked directly above the timestamp in the bottom-left corner)
    if has_custom_title {
        let title_text = title_opt.unwrap().trim();
        let escaped_title = escape_drawtext_str(title_text);

        filters.push(format!(
            "drawtext={}:text='{}':fontcolor={}:fontsize={}:x={}:y=h-th-{}-{}:{}",
            font_param,
            escaped_title,
            fontcolor,
            title_font_size,
            margin,
            margin,
            line_spacing,
            stroke_opts
        ));
    }

    // 2. Date & Time Timestamp Filter (at the bottom-left corner)
    filters.push(format!(
        "drawtext={}:text='{}':fontcolor={}:fontsize={}:x={}:y=h-th-{}:{}",
        font_param, escaped_ts, fontcolor, fontsize, margin, margin, stroke_opts
    ));

    Some(filters.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_drawtext() {
        assert_eq!(escape_drawtext_str("12:34:56"), "12\\:34\\:56");
        assert_eq!(escape_drawtext_str("Terry's Day"), "Terry\\'s Day");
    }

    #[test]
    fn test_build_drawtext_filter_disabled() {
        let cfg = OverlayConfig {
            enabled: false,
            ..Default::default()
        };
        let filter = build_drawtext_filter("1405-05-30_12-33-03", Some("Day One"), &cfg, "640x480");
        assert!(filter.is_none());
    }

    #[test]
    fn test_build_drawtext_filter_enabled() {
        let cfg = OverlayConfig {
            enabled: true,
            style: OverlayStyle::VhsYellow,
            font: "vt323".to_string(),
            font_size: Some(28),
            show_title: true,
        };
        let filter = build_drawtext_filter("1405-05-30_12-33-03", Some("Day One"), &cfg, "640x480");
        assert!(filter.is_some());
        let f = filter.unwrap();
        assert!(f.contains("1405-05-30  12\\:33\\:03"));
        assert!(f.contains("fontcolor=0xFFE500"));
        assert!(f.contains("fontsize=28"));
        assert!(f.contains("borderw="));
        assert!(f.contains("Day One"));
        assert!(f.contains("y=h-th-"));
    }
}
