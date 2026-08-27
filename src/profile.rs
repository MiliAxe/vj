use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub resolution: String,
    pub fps: u32,
    pub vcodec: String,
    pub vpreset: u32,
    pub vcrf: u32,
    pub acodec: String,
    pub achannels: u32,
    pub abitrate: String,
    pub vfilter: Option<String>,
    pub afilter: Option<String>,
    pub extra_flags: Option<String>,
    pub est_10m: Option<String>,
    pub est_1h: Option<String>,
}

impl Profile {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        resolution: &str,
        fps: u32,
        vcodec: &str,
        vpreset: u32,
        vcrf: u32,
        acodec: &str,
        achannels: u32,
        abitrate: &str,
        vfilter: Option<&str>,
        afilter: Option<&str>,
        extra_flags: Option<&str>,
        est_10m: Option<&str>,
        est_1h: Option<&str>,
    ) -> Self {
        Self {
            resolution: resolution.to_string(),
            fps,
            vcodec: vcodec.to_string(),
            vpreset,
            vcrf,
            acodec: acodec.to_string(),
            achannels,
            abitrate: abitrate.to_string(),
            vfilter: vfilter.map(|s| s.to_string()),
            afilter: afilter.map(|s| s.to_string()),
            extra_flags: extra_flags.map(|s| s.to_string()),
            est_10m: est_10m.map(|s| s.to_string()),
            est_1h: est_1h.map(|s| s.to_string()),
        }
    }
}

pub fn get_builtin_profiles() -> HashMap<String, Profile> {
    let mut map = HashMap::new();

    map.insert(
        "potato".to_string(),
        Profile::new(
            "320x240",
            10,
            "libsvtav1",
            4,
            48,
            "libopus",
            1,
            "10k",
            Some("scale=320:240,fps=10,hqdn3d=5:4:7:5,unsharp=3:3:0.5"),
            Some("highpass=f=80,loudnorm=I=-16:TP=-1.5:LRA=11"),
            Some("-svtav1-params tune=0:film-grain=0"),
            Some("~2.0 MB"),
            Some("~12 MB"),
        ),
    );

    map.insert(
        "compact".to_string(),
        Profile::new(
            "480x360",
            12,
            "libsvtav1",
            4,
            44,
            "libopus",
            1,
            "12k",
            Some("scale=480:360,fps=12,hqdn3d=4:3:6:4.5,unsharp=3:3:0.5"),
            Some("highpass=f=80,loudnorm=I=-16:TP=-1.5:LRA=11"),
            Some("-svtav1-params tune=0:film-grain=0"),
            Some("~4.5 MB"),
            Some("~27 MB"),
        ),
    );

    map.insert(
        "terry".to_string(),
        Profile::new(
            "640x480",
            15,
            "libsvtav1",
            4,
            38,
            "libopus",
            1,
            "14k",
            Some("scale=640:480,fps=15,hqdn3d=3:2.5:5:4,unsharp=3:3:0.4"),
            Some("highpass=f=80,loudnorm=I=-16:TP=-1.5:LRA=11"),
            Some("-svtav1-params tune=0:film-grain=0"),
            Some("~8.0 MB"),
            Some("~48 MB"),
        ),
    );

    map.insert(
        "balanced".to_string(),
        Profile::new(
            "1280x720",
            24,
            "libsvtav1",
            5,
            30,
            "libopus",
            2,
            "32k",
            Some("scale=1280:720,fps=24,hqdn3d=1.5:1.5:3:3"),
            Some("highpass=f=80,loudnorm=I=-16:TP=-1.5:LRA=11"),
            Some("-svtav1-params tune=0:film-grain=0"),
            Some("~22 MB"),
            Some("~130 MB"),
        ),
    );

    map.insert(
        "hq".to_string(),
        Profile::new(
            "1920x1080",
            30,
            "libsvtav1",
            6,
            24,
            "libopus",
            2,
            "64k",
            Some("scale=1920:1080,fps=30"),
            Some("loudnorm=I=-16:TP=-1.5:LRA=11"),
            Some("-svtav1-params tune=0:film-grain=0"),
            Some("~60 MB"),
            Some("~360 MB"),
        ),
    );

    map
}

pub fn resolve_profile(
    name: &str,
    custom_profiles: &HashMap<String, Profile>,
) -> (String, Profile) {
    let lower = name.to_lowercase();
    let lower_ref = lower.as_str();

    let resolved_name = match lower_ref {
        "default" => "terry",
        "micro" | "ultra" => "potato",
        other => other,
    };

    if let Some(p) = custom_profiles.get(resolved_name) {
        return (resolved_name.to_string(), p.clone());
    }

    let builtins = get_builtin_profiles();
    if let Some(p) = builtins.get(resolved_name) {
        return (resolved_name.to_string(), p.clone());
    }

    // Fallback to terry
    ("terry".to_string(), builtins.get("terry").unwrap().clone())
}
