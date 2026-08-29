use crate::config::Config;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::Write;
use std::process::{Command, Stdio};

/// All lifecycle events that can trigger hooks.
pub const EVENT_NAMES: &[&str] = &[
    "pre_record",
    "post_record",
    "post_encode",
    "post_import",
    "pre_play",
    "post_play",
    "pre_delete",
    "post_delete",
];

/// A single configured hook command.
///
/// Payload data is exposed to the hook process in two ways:
/// - As environment variables: `VJ_EVENT`, `VJ_ENTRY_ID`, `VJ_ENTRY_DIR`, `VJ_PROFILE`,
///   `VJ_TITLE`, `VJ_TAGS`, `VJ_ENCRYPTED`, `VJ_FILE`
/// - As the full JSON payload piped to stdin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookSpec {
    /// Shell command to execute (run via `sh -c`).
    pub run: String,
    /// If true, a non-zero exit aborts the vj operation with an error.
    /// If false (default), a non-zero exit only prints a warning.
    #[serde(default)]
    pub blocking: bool,
}

pub type HookMap = HashMap<String, Vec<HookSpec>>;

/// Build a JSON payload object with the event name plus arbitrary fields.
pub fn payload(event: &str, fields: &[(&str, Value)]) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("event".into(), json!(event));
    for (k, v) in fields {
        obj.insert((*k).to_string(), v.clone());
    }
    Value::Object(obj)
}

/// Dispatch an event to all configured hooks.
///
/// Blocking hook failures propagate as errors; non-blocking failures warn.
pub fn dispatch(config: &Config, event: &str, payload: &Value) -> Result<()> {
    let hooks: &Vec<HookSpec> = match config.hooks.get(event) {
        Some(h) if !h.is_empty() => h,
        _ => return Ok(()),
    };

    let json = serde_json::to_string(payload).unwrap_or_else(|_| "{}".to_string());

    for (i, hook) in hooks.iter().enumerate() {
        let result = run_hook(hook, event, i, &json);
        match result {
            Ok(()) => {}
            Err(e) if hook.blocking => {
                bail!("Blocking hook aborted the operation: {:#}", e)
            }
            Err(e) => {
                eprintln!("[vj] Warning: {:#}", e);
            }
        }
    }

    Ok(())
}
fn run_hook(hook: &HookSpec, event: &str, index: usize, json: &str) -> Result<()> {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(&hook.run).env("VJ_EVENT", event);

    if let Some(obj) = payload_fields(json) {
        for (k, v) in obj {
            let env_key = format!("VJ_{}", k.to_uppercase());
            let env_val = match v {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                other => other.to_string(),
            };
            cmd.env(env_key, env_val);
        }
    }

    cmd.stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn hook {}[{}]: {}", event, index, hook.run))?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(json.as_bytes());
    }

    let status = child
        .wait()
        .with_context(|| format!("Failed to wait for hook {}[{}]", event, index))?;

    if !status.success() {
        bail!(
            "Hook {}[{}] ({}) exited with {}",
            event,
            index,
            hook.run,
            status
        );
    }

    Ok(())
}

fn payload_fields(json: &str) -> Option<serde_json::Map<String, Value>> {
    serde_json::from_str::<Value>(json)
        .ok()
        .and_then(|v| v.as_object().cloned())
}

use colored::Colorize;
/// `vj hooks` — list all configured hooks.
pub fn list_hooks(config: &Config) {
    println!("vj Hooks (from config.toml):\n");
    let mut any = false;
    for event in EVENT_NAMES {
        if let Some(hooks) = config.hooks.get(*event) {
            if hooks.is_empty() {
                continue;
            }
            any = true;
            println!("  {} ({} hook/s)", event.bold(), hooks.len());
            for hook in hooks {
                println!("    run: {}", hook.run);
                println!("      blocking: {}", hook.blocking);
            }
        }
    }
    if !any {
        println!("  No hooks configured.");
        println!();
        println!("  Add hooks to ~/.config/vj/config.toml, e.g.:");
        println!();
        println!("    [[hooks.post_encode]]");
        println!("    run = \"notify-send vj \\\"Encoded $VJ_ENTRY_ID\\\"\"");
        println!("    blocking = false");
    }
}

/// `vj hooks --test <event>` — fire a test payload so hooks can be debugged.
pub fn test_fire(config: &Config, event: &str) -> Result<()> {
    if !EVENT_NAMES.contains(&event) {
        bail!(
            "Unknown event '{}'. Valid events: {}",
            event,
            EVENT_NAMES.join(", ")
        );
    }
    let p = payload(
        event,
        &[
            ("entry_id", json!("1405-01-01_10-00-00")),
            ("entry_dir", json!("/tmp/vj_test_entry")),
            ("profile", json!("terry")),
            ("title", json!("Hook Test")),
            ("tags", json!(["test"])),
            ("encrypted", json!(false)),
            ("file", Value::Null),
        ],
    );
    println!(
        "Firing test payload for '{}' (JSON on stdin + $VJ_* env vars):",
        event
    );
    dispatch(config, event, &p)?;
    println!("[✓] All hooks for '{}' completed.", event);
    Ok(())
}

/// Convenience: build + dispatch an entry-related event in one call.
#[allow(clippy::too_many_arguments)]
pub fn dispatch_entry(
    config: &Config,
    event: &str,
    entry_id: &str,
    entry_dir: &std::path::Path,
    profile: Option<&str>,
    title: Option<&str>,
    tags: &[String],
    encrypted: bool,
) -> Result<()> {
    let p = payload(
        event,
        &[
            ("entry_id", json!(entry_id)),
            ("entry_dir", json!(entry_dir.display().to_string())),
            ("profile", profile.map(|s| json!(s)).unwrap_or(Value::Null)),
            ("title", title.map(|s| json!(s)).unwrap_or(Value::Null)),
            ("tags", json!(tags)),
            ("encrypted", json!(encrypted)),
            ("file", Value::Null),
        ],
    );
    dispatch(config, event, &p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(hooks: HookMap) -> Config {
        Config {
            hooks,
            ..Config::default()
        }
    }

    #[test]
    fn test_dispatch_no_hooks_is_noop() {
        let cfg = test_config(HookMap::new());
        assert!(dispatch(&cfg, "post_encode", &payload("post_encode", &[])).is_ok());
    }

    #[test]
    fn test_non_blocking_failure_warns_only() {
        let cfg = test_config(
            [(
                "post_encode".to_string(),
                vec![HookSpec {
                    run: "exit 3".to_string(),
                    blocking: false,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert!(dispatch(&cfg, "post_encode", &payload("post_encode", &[])).is_ok());
    }

    #[test]
    fn test_blocking_failure_is_error() {
        let cfg = test_config(
            [(
                "pre_record".to_string(),
                vec![HookSpec {
                    run: "exit 1".to_string(),
                    blocking: true,
                }],
            )]
            .into_iter()
            .collect(),
        );
        assert!(dispatch(&cfg, "pre_record", &payload("pre_record", &[])).is_err());
    }

    #[test]
    fn test_env_and_stdin_received() {
        let out = std::env::temp_dir().join(format!("vj_hook_test_{}", std::process::id()));
        let stdin_out = std::env::temp_dir().join(format!("vj_hook_stdin_{}", std::process::id()));
        let cfg = test_config(
            [(
                "post_encode".to_string(),
                vec![HookSpec {
                    run: format!(
                        "echo \"$VJ_EVENT,$VJ_ENTRY_ID,$VJ_ENCRYPTED\" > {}; cat > {}",
                        out.display(),
                        stdin_out.display()
                    ),
                    blocking: false,
                }],
            )]
            .into_iter()
            .collect(),
        );
        let p = payload(
            "post_encode",
            &[
                ("entry_id", json!("1405-01-01_10-00-00")),
                ("encrypted", json!(true)),
            ],
        );
        assert!(dispatch(&cfg, "post_encode", &p).is_ok());
        let content = std::fs::read_to_string(&out).unwrap();
        assert_eq!(content.trim(), "post_encode,1405-01-01_10-00-00,true");
        let stdin = std::fs::read_to_string(&stdin_out).unwrap();
        let got: Value = serde_json::from_str(&stdin).unwrap();
        assert_eq!(got, p);
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(&stdin_out);
    }

    #[test]
    fn test_test_fire_rejects_unknown_event() {
        let cfg = test_config(HookMap::new());
        assert!(test_fire(&cfg, "bogus_event").is_err());
    }

    #[test]
    fn test_test_fire_accepts_known_event() {
        let cfg = test_config(HookMap::new());
        assert!(test_fire(&cfg, "post_encode").is_ok());
    }
}
