mod calendar;
mod cli;
mod completions;
mod config;
mod crypto;
mod engine;
mod entry;
mod profile;
mod server;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser};
use cli::{Cli, Commands, CompletionTarget};
use config::{get_config_file, Config};
use crypto::GpgAuth;
use entry::{find_entry, load_entries};
use std::fs;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::load().unwrap_or_default();

    if let Some(cmd) = cli.command {
        match cmd {
            Commands::Record(args) => {
                let encrypt_opt = if args.encrypt {
                    Some(true)
                } else if args.no_encrypt {
                    Some(false)
                } else {
                    None
                };

                let opts = engine::record::RecordOptions {
                    profile: args.profile,
                    encrypt: encrypt_opt,
                    title: args.title,
                    tags: args.tags,
                    note: args.note,
                    interactive: args.interactive,
                    verbose: cli.verbose,
                    wait: args.wait,
                };
                engine::record::execute_record(opts, &config)?;
            }

            Commands::Import(args) => {
                let encrypt_opt = if args.encrypt {
                    Some(true)
                } else if args.no_encrypt {
                    Some(false)
                } else {
                    None
                };

                let opts = engine::import::ImportOptions {
                    files: args.files,
                    profile: args.profile,
                    encrypt: encrypt_opt,
                    title: args.title,
                    tags: args.tags,
                    note: args.note,
                    interactive: args.interactive,
                    keep: args.keep,
                    verbose: cli.verbose,
                };
                engine::import::execute_import(opts, &config)?;
            }

            Commands::InboxServer { port } => {
                server::run_inbox_server(port, &config).await?;
            }

            Commands::PreviewInbox { file } => {
                engine::import::preview_inbox_item(&file, config.calendar_system())?;
            }

            Commands::Play { entry_id } => {
                engine::play::execute_play(entry_id, cli.verbose, &config)?;
            }

            Commands::Preview { entry_id } => {
                let entries = load_entries(&config.entries_path(), &config.temp_path())?;
                let entry = find_entry(&entries, &entry_id)
                    .with_context(|| format!("Entry not found: {}", entry_id))?;
                let auth = GpgAuth::from_config(&config);
                entry::print_preview(entry, &auth)?;
            }

            Commands::List { quiet } => {
                let entries = load_entries(&config.entries_path(), &config.temp_path())?;
                entry::print_list(&entries, quiet);
            }

            Commands::Random => {
                engine::play::execute_random(cli.verbose, &config)?;
            }

            Commands::Encrypt { target } => {
                let entries = load_entries(&config.entries_path(), &config.temp_path())?;
                let auth = GpgAuth::from_config(&config);

                let targets: Vec<&entry::Entry> = if target == "all" {
                    entries.iter().collect()
                } else {
                    let e = find_entry(&entries, &target)
                        .with_context(|| format!("Entry not found: {}", target))?;
                    vec![e]
                };

                for e in targets {
                    let v_plain = e.dir.join("video.mkv");
                    let note_plain = e.dir.join("note.md");
                    let meta_plain = e.dir.join("meta.json");

                    if v_plain.exists() {
                        println!("Encrypting {}...", e.id);
                        crypto::encrypt_file(&v_plain, &auth)?;
                        if note_plain.exists() {
                            let _ = crypto::encrypt_file(&note_plain, &auth);
                        }
                        if meta_plain.exists() {
                            if let Ok(content) = fs::read_to_string(&meta_plain) {
                                if let Ok(mut meta) = serde_json::from_str::<entry::Meta>(&content) {
                                    meta.encrypted = true;
                                    if let Ok(new_json) = serde_json::to_string_pretty(&meta) {
                                        let _ = fs::write(&meta_plain, new_json);
                                    }
                                }
                            }
                            let _ = crypto::encrypt_file(&meta_plain, &auth);
                        }
                    }
                }
                println!("Encryption complete.");
            }

            Commands::Decrypt { target } => {
                let entries = load_entries(&config.entries_path(), &config.temp_path())?;
                let auth = GpgAuth::from_config(&config);

                let targets: Vec<&entry::Entry> = if target == "all" {
                    entries.iter().collect()
                } else {
                    let e = find_entry(&entries, &target)
                        .with_context(|| format!("Entry not found: {}", target))?;
                    vec![e]
                };

                for e in targets {
                    let v_gpg = e.dir.join("video.mkv.gpg");
                    let note_gpg = e.dir.join("note.md.gpg");
                    let meta_gpg = e.dir.join("meta.json.gpg");

                    if v_gpg.exists() {
                        println!("Decrypting {}...", e.id);
                        crypto::decrypt_file(&v_gpg, e.dir.join("video.mkv"), &auth)?;
                        if note_gpg.exists() {
                            let _ = crypto::decrypt_file(&note_gpg, e.dir.join("note.md"), &auth);
                        }
                        if meta_gpg.exists() {
                            let meta_plain = e.dir.join("meta.json");
                            let _ = crypto::decrypt_file(&meta_gpg, &meta_plain, &auth);
                            if let Ok(content) = fs::read_to_string(&meta_plain) {
                                if let Ok(mut meta) = serde_json::from_str::<entry::Meta>(&content) {
                                    meta.encrypted = false;
                                    if let Ok(new_json) = serde_json::to_string_pretty(&meta) {
                                        let _ = fs::write(&meta_plain, new_json);
                                    }
                                }
                            }
                        }
                    }
                }
                println!("Decryption complete.");
            }

            Commands::Delete { entry_id, force } => {
                let entries = load_entries(&config.entries_path(), &config.temp_path())?;
                let entry = find_entry(&entries, &entry_id)
                    .with_context(|| format!("Entry not found: {}", entry_id))?;
                entry::delete_entry(entry, force)?;
            }

            Commands::Stats => {
                let entries = load_entries(&config.entries_path(), &config.temp_path())?;
                entry::print_stats(
                    &entries,
                    &config.entries_path(),
                    &config.inbox_path(),
                    config.calendar_system(),
                );
            }

            Commands::Profiles => {
                println!("vj Compression Profiles:\n");
                println!(
                    "{:<12} {:<18} {:<18} {:<18} {:<16} {:<16}",
                    "PROFILE", "RESOLUTION & FPS", "VIDEO (AV1)", "AUDIO (OPUS)", "EST. (10 MIN)", "EST. (1 HOUR)"
                );
                println!(
                    "{:<12} {:<18} {:<18} {:<18} {:<16} {:<16}",
                    "------------", "------------------", "------------------", "------------------", "----------------", "----------------"
                );

                let builtins = profile::get_builtin_profiles();
                for name in &["potato", "compact", "terry", "balanced", "hq"] {
                    if let Some(p) = builtins.get(*name) {
                        let name_display = if *name == "terry" { "terry (*)" } else { name };
                        println!(
                            "{:<12} {:<18} {:<18} {:<18} {:<16} {:<16}",
                            name_display,
                            format!("{} @ {}fps", p.resolution, p.fps),
                            format!("{} CRF {}", p.vcodec, p.vcrf),
                            format!("{} ({})", p.abitrate, p.acodec),
                            p.est_10m.as_deref().unwrap_or("~"),
                            p.est_1h.as_deref().unwrap_or("~")
                        );
                    }
                }

                if !config.profiles.is_empty() {
                    println!("\nUser-Defined Profiles (from {:?}):", get_config_file());
                    println!(
                        "{:<12} {:<18} {:<18} {:<18} {:<16} {:<16}",
                        "PROFILE", "RESOLUTION & FPS", "VIDEO (AV1)", "AUDIO (OPUS)", "EST. (10 MIN)", "EST. (1 HOUR)"
                    );
                    println!(
                        "{:<12} {:<18} {:<18} {:<18} {:<16} {:<16}",
                        "------------", "------------------", "------------------", "------------------", "----------------", "----------------"
                    );
                    for (name, p) in &config.profiles {
                        println!(
                            "{:<12} {:<18} {:<18} {:<18} {:<16} {:<16}",
                            name,
                            format!("{} @ {}fps", p.resolution, p.fps),
                            format!("{} CRF {}", p.vcodec, p.vcrf),
                            format!("{} ({})", p.abitrate, p.acodec),
                            p.est_10m.as_deref().unwrap_or("Custom"),
                            p.est_1h.as_deref().unwrap_or("Custom")
                        );
                    }
                }

                println!("\n(*) Default profile. Change with default_profile = \"...\" in {:?}", get_config_file());
            }

            Commands::Config => {
                let config_file = get_config_file();
                if !config_file.exists() {
                    let _ = config.save(&config_file);
                }
                let mut ed = Command::new(&config.editor);
                ed.arg(&config_file);
                let _ = ed.status();
            }

            Commands::Completions { target } => {
                let mut cmd = Cli::command();
                match target {
                    CompletionTarget::Bash => completions::print_completions(clap_complete::Shell::Bash, &mut cmd),
                    CompletionTarget::Zsh => completions::print_completions(clap_complete::Shell::Zsh, &mut cmd),
                    CompletionTarget::Fish => completions::print_completions(clap_complete::Shell::Fish, &mut cmd),
                    CompletionTarget::Powershell => completions::print_completions(clap_complete::Shell::PowerShell, &mut cmd),
                    CompletionTarget::Elvish => completions::print_completions(clap_complete::Shell::Elvish, &mut cmd),
                    CompletionTarget::Install => completions::install_completions(&mut cmd)?,
                }
            }

            Commands::InternalEncode {
                temp_raw,
                entry_dir,
                profile,
                encrypt,
            } => {
                let (_, profile_spec) = profile::resolve_profile(&profile, &config.profiles);
                engine::encode::run_encoding(
                    &temp_raw,
                    &entry_dir,
                    &profile_spec,
                    encrypt,
                    &config,
                    false,
                )?;
            }
        }
    } else if let Some(target) = cli.target {
        engine::play::execute_play(Some(target), cli.verbose, &config)?;
    } else {
        let mut cmd = Cli::command();
        cmd.print_help()?;
        println!();
    }

    Ok(())
}
