<div align="center">

# `vj` (Rust Edition)

### *Ultra-Fast, Minimal, Ultra-Compressed & Secure Video Journaling*

[![Made with Rust](https://img.shields.io/badge/Made%20with-Rust-orange?style=for-the-badge&logo=rust)](https://www.rust-lang.org/)
[![Made with AI](https://img.shields.io/badge/Made%20with-AI-lightgrey?style=for-the-badge)](https://github.com/mefengl/made-by-ai)

This project is inspired by the King himself, [Terry A. Davis](https://en.wikipedia.org/wiki/Terry_A._Davis) (RIP). I noticed that he had most of his career and life on tape. I thought to myself, maybe it would be nice to have a utility to record your journal entries talking. Not a bad idea huh? This tool was made with AI (Sorry Terry, you probably wouldn't like me writing a tool like this using AI). This tool was originally written with [bash](https://github.com/MiliAxe/vj), until I realized it was getting too big and the speed was suffering. 

</div>

---

## Highlights & Features

- **⚡ Blazing Fast Rust Core**: Instant sub-millisecond CLI startup, zero Python dependency, zero interpreter overhead.
- **🖼️ In-Terminal Storyboard & Interactive MPV Peek**: Fast in-terminal 2x2 storyboard previews in `fzf` via `chafa`, plus instant floating video peek (<kbd>Ctrl-P</kbd> / <kbd>Space</kbd> inside `fzf`).
- **📼 Optional Retro VHS / Camcorder OSD Overlay**: Opt-in authentic on-screen timestamp and title stacked in the bottom-left corner with embedded retro fonts (`vt323`, `silkscreen`, `press_start_2p`, `share_tech_mono`).
- **📼 Ultra-Compact SVT-AV1 + Opus**: Compress hours of speech video into negligible disk space (~15 MB per hour in `terry` mode).
- **🔒 Zero-Disk-Leak RAM Streaming**: Encrypted videos pipe directly through RAM into `mpv` (`gpg -d | mpv -`) without writing plaintext to disk.
- **🚀 Zero-Friction Recording**: Run `vj record` and close preview to save. Background encoding runs silently via low OS priority (`ionice`/`nice`).
- **📅 Native Jalali (Solar Hijri) & Gregorian**: Seamless calendar timestamps calculated natively in Rust with zero lag.
- **📱 Built-In Mobile Web Upload Server**: Native async HTTP server with embedded drag-and-drop web UI and in-terminal ANSI QR code (`vj inbox-server`).
- **🔍 Vim-First `fzf` Browser**: Interactive browsing with live metadata, note, and storyboard contact sheet preview panes.
- **🗑️ Batch Deletion & Interactive `fzf` Vault Management**: Multi-select deletion with live preview panes (`vj delete`) or batch deletion by IDs (`vj delete id1 id2 ...`).
- **⚙️ Modern TOML Configuration**: Clean XDG-standard configuration at `~/.config/vj/config.toml`.
- **🐚 Auto Shell Completions**: Native completions for Fish, Bash, Zsh, PowerShell, and Elvish via `clap_complete`.

---

## Installation

```bash
# Clone and enter the repository
cd vj-rs

# Build and install to ~/.local/bin with shell completions
make install

# System-wide install (optional)
sudo make install PREFIX=/usr/local
```

To uninstall:

```bash
make uninstall
```

---

## Storyboard Previews & Interactive Video Peek in `fzf`

`vj` features an instant preview workflow inside `fzf` (`vj play` / `vj delete`):

1. **In-Terminal 2x2 Storyboard**:
   - In `vj play`, `vj delete`, or `vj preview <id>`, entries display a 4-frame contact sheet rendered directly inside the terminal cells using `chafa` (or `timg`/`viu`).
2. **Interactive Floating Video Peek**:
   - While browsing in `vj play` or `vj delete`, press <kbd>Ctrl-P</kbd> or <kbd>Space</kbd> to pop up a floating, borderless muted video loop in the corner of your screen. Press <kbd>q</kbd> or <kbd>Esc</kbd> to dismiss.

---

## Retro Fonts & OSD Overlay *(Disabled by Default)*

All overlay features are completely **disabled by default**. When you want the retro camcorder aesthetic, pass `-O` / `--overlay` or enable `retro_overlay = true` in `config.toml`.

| Font Identifier | Style / Era | Description |
| :--- | :--- | :--- |
| **`vt323`** *(default)* | DEC VT323 CRT / VHS | Iconic tall retro VHS & CRT phosphor terminal font |
| **`silkscreen`** | 90s Handheld Camcorder | Ultra-crisp pixel matrix font, ideal for compact/potato |
| **`press_start_2p`** | 8-Bit Arcade / Micro | Classic 1980s retro gaming & computer pixel typography |
| **`share_tech_mono`** | Cyberpunk HUD / Sci-Fi | Modern vintage high-tech monospace HUD display font |

View all recommended fonts and styles:
```bash
vj fonts
```

### Overlay Layout:
- **Bottom-Left Corner (Stacked)**:
  - Top Line: Custom entry title (e.g. `Trip to Japan`)
  - Bottom Line: Date and time timestamp (e.g. `1405-05-30  18:12:05`)

### Recording with Retro Overlay:

```bash
# Clean recording (no overlay by default)
vj record

# Opt-in to retro OSD overlay
vj record -O

# Record with custom title (stacked right above timestamp in bottom-left)
vj record -O -t "Trip to Japan"

# Record with custom font size (e.g. 28px)
vj record -O --font-size 28

# Record with 90s camcorder pixel font & white styling
vj record -O --overlay-font silkscreen --overlay-style camcorder_white --font-size 18
```

---

## Command Reference

| Command | Description | Example |
| :--- | :--- | :--- |
| **`vj record`** | Start live webcam capture & preview | `vj record -p terry` |
| **`vj record -t "..."`** | Record with title, tags, or notes | `vj record -t "Life Update" --tags "dev,log" -n` |
| **`vj record -O`** | Record with retro OSD date/time overlay | `vj record -O --overlay-font silkscreen --font-size 20` |
| **`vj import`** | Multi-select import from inbox with video preview pane | `vj import` |
| **`vj import [files...]`** | Import specific videos with metadata conversion | `vj import ~/Downloads/vid.mp4 -t "Trip"` |
| **`vj inbox-server`** | Start local upload server with phone QR code | `vj inbox-server 8080` |
| **`vj play`** | Interactive `fzf` browser with live metadata preview | `vj play` |
| **`vj play <id>`** | Play specific entry directly in `mpv` | `vj play 1405-05-30_12-33-03` |
| **`vj preview <id>`** | Print metadata, note, and terminal storyboard | `vj preview 1405-05-30_12-33-03` |
| **`vj preview-inbox <file>`** | Inspect format, resolution, codec, and duration | `vj preview-inbox ~/video.mp4` |
| **`vj list`** | List entries in formatted table (`-q` for raw IDs) | `vj list -q` |
| **`vj random`** | Jump into a random historical recording | `vj random` |
| **`vj delete`** | Interactive `fzf` multi-select browser to delete entries | `vj delete` |
| **`vj delete [ids...]`** | Batch delete one or multiple entries by ID | `vj delete 1405-05-30_12-33-03 1405-05-30_14-00-00` |
| **`vj delete -f [ids...]`** | Delete entries without confirmation prompt | `vj delete -f 1405-05-30_12-33-03` |
| **`vj encrypt <id\|all>`** | Encrypt entry or whole vault with AES-256 | `vj encrypt all` |
| **`vj decrypt <id\|all>`** | Decrypt entry or whole vault to plaintext | `vj decrypt all` |
| **`vj stats`** | Display streak, total entries, and storage footprint | `vj stats` |
| **`vj profiles`** | List available built-in & custom compression profiles | `vj profiles` |
| **`vj fonts`** | List recommended retro fonts and styles | `vj fonts` |
| **`vj config`** | Open configuration in `$EDITOR` (`nvim`/`vim`) | `vj config` |
| **`vj completions`** | Output or auto-install shell completions | `vj completions install` |

---

## Compression Profiles

| Profile | Resolution & FPS | Video Codec & Settings | Audio (Opus) | Est. (10 min) | Est. (1 hour) |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **`potato`** | 320×240 @ 10fps | SVT-AV1 (CRF 48, `hqdn3d`, `unsharp`) | 10 kbps Mono VoIP | **~2.0 MB** | **~12 MB** |
| **`compact`** | 480×360 @ 12fps | SVT-AV1 (CRF 44, `hqdn3d`, `unsharp`) | 12 kbps Mono VoIP | **~4.5 MB** | **~27 MB** |
| **`terry`** *(default)* | 640×480 @ 15fps | SVT-AV1 (CRF 38, `hqdn3d`, `unsharp`) | 14 kbps Mono VoIP | **~8.0 MB** | **~48 MB** |
| **`balanced`** | 1280×720 @ 24fps | SVT-AV1 (CRF 30, `hqdn3d`) | 32 kbps Stereo | **~22 MB** | **~130 MB** |
| **`hq`** | 1920×1080 @ 30fps | SVT-AV1 (CRF 24) | 64 kbps Stereo | **~60 MB** | **~360 MB** |

---

## Configuration (`~/.config/vj/config.toml`)

```toml
# Storage directory for journal entries
journal_dir = "~/Videos/Journal/entries"

# Inbox directory for incoming mobile uploads
inbox_dir = "~/Videos/Journal/inbox"

# Calendar system: "jalali" (1405-05-30) or "gregorian" (2026-08-22)
date_calendar = "jalali"

# Default compression profile ("terry", "potato", "compact", "balanced", "hq")
default_profile = "terry"

# Retro OSD Overlay Settings (disabled by default)
retro_overlay = false                  # Overlays are completely OFF by default
overlay_font = "vt323"                 # "vt323", "silkscreen", "press_start_2p", "share_tech_mono", or font path
# overlay_font_size = 24               # Custom font size in pixels (default: auto proportional to resolution)
overlay_style = "vhs_yellow"           # "vhs_yellow", "camcorder_white", "green", "amber", "cyan"
overlay_show_title = true              # When overlay is enabled, show custom title stacked above date

# Hardware capture devices
camera_dev = "/dev/video0"
audio_src = "default"
editor = "nvim"
inbox_port = 8080

# Keyless encryption (optional):
# key_file = "~/.config/vj/key"
# passphrase = ""

# Custom Profiles
[profiles.retro]
resolution = "320x240"
fps = 10
vcodec = "libsvtav1"
vpreset = 4
vcrf = 48
acodec = "libopus"
achannels = 1
abitrate = "10k"
vfilter = "scale=320:240,fps=10,hqdn3d=5:4:7:5,unsharp=3:3:0.5"
afilter = "highpass=f=80,loudnorm=I=-16:TP=-1.5:LRA=11"
extra_flags = "-svtav1-params tune=0:film-grain=0"
```
