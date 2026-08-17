<div align="center">

# `vj`

### *Minimal, Ultra-Compressed & Secure Video Journaling for Hackers*

[![Made with AI](https://img.shields.io/badge/Made%20with-AI-lightgrey?style=for-the-badge)](https://github.com/mefengl/made-by-ai)

This project is inspired by the King himself, [Terry A. Davis](https://en.wikipedia.org/wiki/Terry_A._Davis) (RIP). I noticed that he had most of his career and life on tape. I thought to myself, maybe it would be nice to have a utility to record your journal entries talking. Not a bad idea huh? This tool was made with AI (Sorry Terry, you probably wouldn't like me writing a tool like this using AI).

</div>

---

## Installation

`vj` includes a `Makefile` that installs the executable and configures shell completions for **Fish**, **Bash**, and **Zsh**:

```bash
# User install to ~/.local/bin and shell completion directories
make install

# System-wide install
sudo make install PREFIX=/usr/local
```

To uninstall:

```bash
make uninstall
```

---

## Features

- **Ultra-Compact SVT-AV1 + Opus**: Compress hours of speech video into negligible disk space (~15 MB per hour in `terry` mode).
- **Zero-Disk-Leak RAM Streaming**: Encrypted videos pipe directly through RAM into `mpv` (`gpg -d | mpv -`) without writing plaintext to disk.
- **Zero-Friction Recording**: Run `vj record` and close preview to save. Background encoding runs silently via `ionice`/`nice`.
- **Native Jalali (Solar Hijri) & Gregorian**: Seamless calendar timestamps using native Linux `jdate`.
- **Vim-First `fzf` Browser**: Interactive browsing with live metadata & markdown thought note preview panes.
- **Auto Shell Completions**: Embedded completions for Fish, Bash, and Zsh.
- **XDG Standard Configuration**: Clean configuration located at `~/.config/vj/config.env`.

---

## Quick Start

```bash
# 1. Start recording immediately (default profile, instant save)
vj record

# 2. Browse & stream entries (with Vim preview pane & mpv playback)
vj play

# 3. List entries in clean stdout table
vj list

# 4. Play a random past memory
vj random

# 5. Check total hours and storage stats
vj stats
```

---

## Command Reference

| Command | Description | Example |
| :--- | :--- | :--- |
| **`vj record`** | Start live webcam capture & preview | `vj record -p terry` |
| **`vj record -t "..."`** | Record with title, tags, or notes | `vj record -t "Life Update" --tags "dev,log" -n` |
| **`vj play`** | Interactive `fzf` browser with live metadata preview | `vj play` |
| **`vj play <id>`** | Play specific entry directly in `mpv` | `vj play 1405-05-26_18-35-09` |
| **`vj preview <id>`** | Print entry metadata and note preview to stdout | `vj preview 1405-05-26_18-35-09` |
| **`vj list`** | List entries in formatted table (`-q` for raw IDs) | `vj list -q` |
| **`vj random`** | Jump into a random historical recording | `vj random` |
| **`vj encrypt <id\|all>`** | Encrypt entry or whole vault with AES-256 | `vj encrypt all` |
| **`vj decrypt <id\|all>`** | Decrypt entry or whole vault to plaintext | `vj decrypt all` |
| **`vj stats`** | Display streak, total entries, and storage footprint | `vj stats` |
| **`vj config`** | Open configuration in `$EDITOR` (`nvim`/`vim`) | `vj config` |
| **`vj completions`** | Output or auto-install shell completions | `vj completions install` |

> [!TIP]
> Pass **`-v`** or **`--verbose`** to any command (`vj record -v`, `vj play <id> -v`) to inspect detailed FFmpeg or MPV logs.

---

## Compression Profiles

| Profile | Resolution & FPS | Video Codec & CRF | Audio Codec & Bitrate | Approx. Size |
| :--- | :--- | :--- | :--- | :--- |
| **`terry`** *(default)* | 640×480 @ 24fps | SVT-AV1 (Preset 4, CRF 34, `hqdn3d`) | Opus Mono (20 kbps speech) | **~10–15 MB / hour** |
| **`balanced`** | 1280×720 @ 25fps | SVT-AV1 (Preset 6, CRF 28, `hqdn3d`) | Opus Stereo (48 kbps) | **~60–90 MB / hour** |
| **`hq`** | 1920×1080 @ 30fps | SVT-AV1 (Preset 6, CRF 22) | Opus Stereo (96 kbps) | **~250–400 MB / hour** |

---

## Configuration

Configuration is stored at [`~/.config/vj/config.env`](file:///home/mili/.config/vj/config.env):

```bash
# Storage directory for all video journal entries
JOURNAL_DIR="$HOME/Videos/Journal/entries"

# Calendar system: 'jalali' (1405-05-26 via native jdate) or 'gregorian' (2026-08-17)
DATE_CALENDAR="jalali"

# Keyless encryption (avoids entering passphrase repeatedly):
# KEY_FILE="$HOME/.config/vj/key"
# VJ_PASSPHRASE=""

# Default compression profile ('terry', 'balanced', 'hq')
DEFAULT_PROFILE="terry"

# Hardware capture devices
CAMERA_DEV="/dev/video0"
AUDIO_SRC="default"
EDITOR="nvim"
```

---

## Storage Structure

Each entry is organized in its own isolated folder under `entries/`:

```text
entries/
└── 1405-05-26_18-35-09/
    ├── video.mkv        # (or video.mkv.gpg if encrypted)
    ├── meta.json        # (or meta.json.gpg)
    └── note.md          # (or note.md.gpg)
```
