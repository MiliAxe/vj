use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "vj",
    about = "Minimal, Ultra-Compressed & Secure Video Journaling",
    version,
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, global = true, help = "Show verbose ffmpeg / mpv logs")]
    pub verbose: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(
        name = "record",
        aliases = ["rec", "r"],
        about = "Record a new video entry from webcam"
    )]
    Record(RecordArgs),

    #[command(
        name = "import",
        aliases = ["imp"],
        about = "Import video files from inbox or file paths"
    )]
    Import(ImportArgs),

    #[command(
        name = "inbox-server",
        aliases = ["server", "srv"],
        about = "Start local mobile upload web server with QR code"
    )]
    InboxServer {
        #[arg(help = "Port to listen on (default from config or 8080)")]
        port: Option<u16>,
    },

    #[command(
        name = "preview-inbox",
        about = "Display format, resolution, codec, duration for an inbox video"
    )]
    PreviewInbox {
        #[arg(help = "Path to inbox video file")]
        file: PathBuf,
    },

    #[command(
        name = "play",
        aliases = ["watch", "p", "w"],
        about = "Play entry in mpv with zero-disk-leak RAM streaming"
    )]
    Play {
        #[arg(help = "Entry timestamp/ID (interactive fzf if omitted)")]
        entry_id: Option<String>,
    },

    #[command(
        name = "preview",
        about = "Display metadata, size, status, note, and storyboard for an entry"
    )]
    Preview {
        #[arg(help = "Entry timestamp/ID")]
        entry_id: String,
    },

    #[command(
        name = "list",
        aliases = ["ls", "l"],
        about = "List all journal entries in chronological table format"
    )]
    List {
        #[arg(short, long, help = "Output only entry IDs")]
        quiet: bool,
    },

    #[command(
        name = "random",
        aliases = ["rand"],
        about = "Play a random past memory entry"
    )]
    Random,

    #[command(
        name = "encrypt",
        aliases = ["enc"],
        about = "Encrypt specified entry or all entries with GPG AES-256"
    )]
    Encrypt {
        #[arg(help = "Entry ID or 'all'")]
        target: String,
    },

    #[command(
        name = "decrypt",
        aliases = ["dec"],
        about = "Decrypt specified entry or all entries to plaintext"
    )]
    Decrypt {
        #[arg(help = "Entry ID or 'all'")]
        target: String,
    },

    #[command(
        name = "delete",
        aliases = ["del", "rm", "remove"],
        about = "Permanently delete one or more entries (opens fzf multi-select if empty)"
    )]
    Delete {
        #[arg(help = "Entry IDs to delete (opens fzf multi-select if empty)")]
        entry_ids: Vec<String>,
        #[arg(short, long, help = "Skip confirmation prompt")]
        force: bool,
    },

    #[command(
        name = "stats",
        aliases = ["stat", "s"],
        about = "Display storage and recording summary"
    )]
    Stats,

    #[command(
        name = "profiles",
        aliases = ["profile"],
        about = "List all available compression profiles and estimated disk footprints"
    )]
    Profiles,

    #[command(
        name = "config",
        aliases = ["cfg"],
        about = "Open configuration file in $EDITOR"
    )]
    Config,

    #[command(
        name = "fonts",
        aliases = ["font"],
        about = "List recommended retro fonts for the OSD overlay"
    )]
    Fonts,

    #[command(
        name = "completions",
        aliases = ["completion"],
        about = "Generate or install shell completion scripts"
    )]
    Completions {
        #[arg(
            value_enum,
            default_value = "install",
            help = "Shell type or 'install' to auto-install"
        )]
        target: CompletionTarget,
    },

    #[command(hide = true, name = "__encode")]
    InternalEncode {
        temp_raw: PathBuf,
        entry_dir: PathBuf,
        profile: String,
        #[arg(long)]
        encrypt: bool,
        #[arg(long)]
        denoise: bool,
        #[arg(long)]
        overlay: bool,
        #[arg(long)]
        overlay_style: Option<String>,
        #[arg(long)]
        overlay_font: Option<String>,
        #[arg(long)]
        overlay_font_size: Option<u32>,
        #[arg(long)]
        overlay_title: bool,
    },

    #[command(hide = true, name = "__peek")]
    InternalPeek {
        entry_id: String,
    },
}

#[derive(Args, Debug)]
pub struct RecordArgs {
    #[arg(short, long, help = "Compression profile (potato, compact, terry, balanced, hq, or custom)")]
    pub profile: Option<String>,

    #[arg(short, long, help = "Encrypt with GPG AES-256")]
    pub encrypt: bool,

    #[arg(long, help = "Save unencrypted")]
    pub no_encrypt: bool,

    #[arg(short, long, help = "Title of entry")]
    pub title: Option<String>,

    #[arg(long, help = "Comma-separated tags")]
    pub tags: Option<String>,

    #[arg(short, long, help = "Open editor for notes")]
    pub note: bool,

    #[arg(short, long, help = "Interactive prompt for title and note")]
    pub interactive: bool,

    #[arg(long, aliases = ["no-bg"], help = "Encode in foreground instead of background")]
    pub wait: bool,

    #[arg(short = 'D', long, help = "Enable microphone background noise suppression (afftdn)")]
    pub denoise: bool,

    #[arg(long, help = "Disable microphone background noise suppression")]
    pub no_denoise: bool,

    #[arg(short = 'O', long, help = "Enable retro VHS/camcorder OSD date overlay")]
    pub overlay: bool,

    #[arg(long, help = "Disable retro OSD date overlay")]
    pub no_overlay: bool,

    #[arg(long, help = "Retro OSD style (vhs_yellow, camcorder_white, green, amber, cyan)")]
    pub overlay_style: Option<String>,

    #[arg(long, help = "Retro OSD font (vt323, silkscreen, press_start_2p, share_tech_mono, or font path/name)")]
    pub overlay_font: Option<String>,

    #[arg(long, aliases = ["font-size"], help = "Retro OSD font size (default: auto proportional)")]
    pub overlay_font_size: Option<u32>,

    #[arg(long, help = "Include entry title in retro OSD overlay")]
    pub overlay_title: bool,

    #[arg(long, help = "Do not include entry title in retro OSD overlay")]
    pub no_overlay_title: bool,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    #[arg(help = "Video files to import (opens fzf multi-select if empty)")]
    pub files: Vec<PathBuf>,

    #[arg(short, long, help = "Compression profile")]
    pub profile: Option<String>,

    #[arg(short, long, help = "Encrypt with GPG AES-256")]
    pub encrypt: bool,

    #[arg(long, help = "Save unencrypted")]
    pub no_encrypt: bool,

    #[arg(short, long, help = "Title of entry")]
    pub title: Option<String>,

    #[arg(long, help = "Comma-separated tags")]
    pub tags: Option<String>,

    #[arg(short, long, help = "Open editor for notes")]
    pub note: bool,

    #[arg(short, long, help = "Prompt for title and note")]
    pub interactive: bool,

    #[arg(long, help = "Keep raw file in inbox")]
    pub keep: bool,

    #[arg(short = 'D', long, help = "Enable microphone background noise suppression (afftdn)")]
    pub denoise: bool,

    #[arg(long, help = "Disable microphone background noise suppression")]
    pub no_denoise: bool,

    #[arg(short = 'O', long, help = "Enable retro VHS/camcorder OSD date overlay")]
    pub overlay: bool,

    #[arg(long, help = "Disable retro OSD date overlay")]
    pub no_overlay: bool,

    #[arg(long, help = "Retro OSD style (vhs_yellow, camcorder_white, green, amber, cyan)")]
    pub overlay_style: Option<String>,

    #[arg(long, help = "Retro OSD font (vt323, silkscreen, press_start_2p, share_tech_mono, or font path/name)")]
    pub overlay_font: Option<String>,

    #[arg(long, aliases = ["font-size"], help = "Retro OSD font size (default: auto proportional)")]
    pub overlay_font_size: Option<u32>,

    #[arg(long, help = "Include entry title in retro OSD overlay")]
    pub overlay_title: bool,

    #[arg(long, help = "Do not include entry title in retro OSD overlay")]
    pub no_overlay_title: bool,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompletionTarget {
    Bash,
    Zsh,
    Fish,
    Powershell,
    Elvish,
    Install,
}
