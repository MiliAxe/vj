# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-08-26

### Added
- **Ultra-Compressed AV1 Video Engine**: Background encoding pipeline with SVT-AV1 + Opus (`potato`, `compact`, `terry`, `balanced`, `hq` profiles).
- **Asynchronous Non-Blocking Processing**: Background encoding runs via low CPU and I/O scheduling (`nice` + `ionice`), keeping terminal responsive.
- **Privacy & Encryption**: On-disk AES-256 GPG encryption with zero-disk-leak streaming directly into RAM (`gpg -d | mpv -`).
- **Interactive TUI Browsing**: `fzf` integration for `vj play` and `vj delete` with multi-select and live metadata/note preview panes.
- **In-Terminal Storyboard Previews**: 2x2 contact sheet generation and rendering inside terminal cells via `chafa`, `timg`, or `viu`.
- **Interactive Floating Video Peek**: Instant popup loop video playback (<kbd>Ctrl-P</kbd> / <kbd>Space</kbd>) inside `fzf`.
- **Retro CRT Contribution Heatmap**: Visual activity grid with streaks and phosphor shading (`░`, `▒`, `▓`, `█`) in `vj stats` (`-m 3|6|12`, `-y`).
- **Dual Calendar Engine**: Native bidirectional Jalali (Solar Hijri) and Gregorian calendar normalization.
- **Optional Retro VHS & Camcorder OSD Overlay**: Authentic on-screen timestamp and stacked title with embedded typography (`vt323`, `silkscreen`, `press_start_2p`, `share_tech_mono`).
- **Audio Noise Reduction**: Optional microphone noise suppression filter (`afftdn`) via CLI flag (`-D` / `--denoise`) and `config.toml`.
- **Mobile Inbox HTTP Web Server**: Built-in async web server with QR code generation for uploading recordings from mobile phones (`vj inbox-server`).
- **Automated Shell Completions**: Shell completions for Fish, Bash, Zsh, PowerShell, and Elvish.
- **Linux Multi-Arch Release CI**: Cross-compilation workflow building standalone binaries for `x86_64` and `aarch64`.

### Added (Unreleased)
- **Lifecycle Hook System**: User-defined shell hooks for `pre_record`, `post_record`, `post_encode`, `post_import`, `pre_play`, `post_play`, `pre_delete`, and `post_delete` events, configured via `[[hooks.<event>]]` tables in `config.toml`. Hooks receive the event payload as `VJ_*` environment variables and JSON on stdin; `blocking = true` hooks can abort operations. Includes `vj hooks` listing and `vj hooks --test <event>` dry-run firing.
