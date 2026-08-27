use anyhow::Result;
use clap::Command;
use clap_complete::{generate, Shell};
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;

struct SafeStdout;

impl Write for SafeStdout {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match io::stdout().write(buf) {
            Ok(n) => Ok(n),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(buf.len()),
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match io::stdout().flush() {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
            Err(e) => Err(e),
        }
    }
}

pub fn get_fish_completion() -> &'static str {
    r#"# Fish completion for vj

set -l commands record import inbox-server preview-inbox play preview list random encrypt decrypt delete stats profiles fonts config completions help

complete -c vj -f
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a record -d "Record a new video entry"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a play -d "Play entry in mpv (RAM streaming for encrypted)"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a preview -d "Preview entry metadata, note, and storyboard"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a import -d "Import videos from inbox or file paths"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a inbox-server -d "Start local mobile upload server with QR code"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a preview-inbox -d "Display format and details for an inbox video"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a list -d "List all entries in table format"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a random -d "Play a random past entry"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a encrypt -d "Encrypt entry with GPG AES-256"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a decrypt -d "Decrypt entry to plaintext"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a delete -d "Permanently delete entries"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a stats -d "Show summary stats and storage"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a profiles -d "List available compression profiles"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a fonts -d "List recommended retro fonts for OSD"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a config -d "Edit configuration file"
complete -c vj -n "not __fish_seen_subcommand_from $commands" -a completions -d "Output or install shell completions"

# Helper for dynamic entry IDs
function __fish_vj_entries
    vj list -q 2>/dev/null
end

# Record & Import options
complete -c vj -n "__fish_seen_subcommand_from record import" -s p -l profile -x -a "potato compact terry balanced hq" -d "Compression profile"
complete -c vj -n "__fish_seen_subcommand_from record import" -s e -l encrypt -d "Encrypt with GPG AES-256"
complete -c vj -n "__fish_seen_subcommand_from record import" -l no-encrypt -d "Save unencrypted"
complete -c vj -n "__fish_seen_subcommand_from record import" -s t -l title -r -d "Title of entry"
complete -c vj -n "__fish_seen_subcommand_from record import" -l tags -r -d "Comma-separated tags"
complete -c vj -n "__fish_seen_subcommand_from record import" -s n -l note -d "Open editor for notes"
complete -c vj -n "__fish_seen_subcommand_from record import" -s i -l interactive -d "Prompt for title and note"
complete -c vj -n "__fish_seen_subcommand_from record" -l wait -l no-bg -d "Encode in foreground"
complete -c vj -n "__fish_seen_subcommand_from record import" -s D -l denoise -d "Enable microphone noise reduction (afftdn)"
complete -c vj -n "__fish_seen_subcommand_from record import" -l no-denoise -d "Disable microphone noise reduction"
complete -c vj -n "__fish_seen_subcommand_from record import" -s O -l overlay -d "Enable retro OSD overlay"
complete -c vj -n "__fish_seen_subcommand_from record import" -l no-overlay -d "Disable retro OSD overlay"
complete -c vj -n "__fish_seen_subcommand_from record import" -l overlay-style -x -a "vhs_yellow camcorder_white green amber cyan" -d "Retro OSD color style"
complete -c vj -n "__fish_seen_subcommand_from record import" -l overlay-font -x -a "vt323 silkscreen press_start_2p share_tech_mono" -d "Retro OSD font"
complete -c vj -n "__fish_seen_subcommand_from record import" -l font-size -l overlay-font-size -d "Retro OSD font size in pixels"
complete -c vj -n "__fish_seen_subcommand_from record import" -l overlay-title -d "Show title in overlay"
complete -c vj -n "__fish_seen_subcommand_from record import" -l no-overlay-title -d "Do not show title in overlay"
complete -c vj -n "__fish_seen_subcommand_from record import" -s v -l verbose -d "Verbose output"
complete -c vj -n "__fish_seen_subcommand_from import" -l keep -d "Keep original file in inbox"

# Subcommands taking entry IDs
complete -c vj -n "__fish_seen_subcommand_from play preview delete" -a "(__fish_vj_entries)" -d "Entry ID"
complete -c vj -n "__fish_seen_subcommand_from delete" -s f -l force -d "Skip confirmation"
complete -c vj -n "__fish_seen_subcommand_from encrypt decrypt" -a "all (__fish_vj_entries)" -d "Entry ID or 'all'"
complete -c vj -n "__fish_seen_subcommand_from stats" -s m -l months -x -a "3 6 12" -d "Number of months to display in heatmap"
complete -c vj -n "__fish_seen_subcommand_from stats" -s y -l year -d "Display 12 months (1 year) in heatmap"
complete -c vj -n "__fish_seen_subcommand_from completions" -a "fish bash zsh powershell elvish install" -d "Shell type or install"
"#
}

pub fn get_bash_completion() -> &'static str {
    r#"_vj_completions() {
    local cur prev words cword
    _init_completion || return

    local commands="record play preview list random encrypt decrypt delete stats profiles fonts config import inbox-server preview-inbox completions help"
    local profiles="potato compact terry balanced hq"
    local styles="vhs_yellow camcorder_white green amber cyan"
    local fonts="vt323 silkscreen press_start_2p share_tech_mono"

    if [[ $cword -eq 1 ]]; then
        COMPREPLY=( $(compgen -W "${commands}" -- "${cur}") )
        return 0
    fi

    case "${words[1]}" in
        record)
            if [[ "${prev}" == "-p" || "${prev}" == "--profile" ]]; then
                COMPREPLY=( $(compgen -W "${profiles}" -- "${cur}") )
            elif [[ "${prev}" == "--overlay-style" ]]; then
                COMPREPLY=( $(compgen -W "${styles}" -- "${cur}") )
            elif [[ "${prev}" == "--overlay-font" ]]; then
                COMPREPLY=( $(compgen -W "${fonts}" -- "${cur}") )
            else
                COMPREPLY=( $(compgen -W "-p --profile -e --encrypt --no-encrypt -t --title --tags -n --note -i --interactive --wait --no-bg -D --denoise --no-denoise -O --overlay --no-overlay --overlay-style --overlay-font --font-size --overlay-font-size --overlay-title --no-overlay-title -v --verbose ${profiles}" -- "${cur}") )
            fi
            ;;
        import)
            if [[ "${prev}" == "-p" || "${prev}" == "--profile" ]]; then
                COMPREPLY=( $(compgen -W "${profiles}" -- "${cur}") )
            elif [[ "${prev}" == "--overlay-style" ]]; then
                COMPREPLY=( $(compgen -W "${styles}" -- "${cur}") )
            elif [[ "${prev}" == "--overlay-font" ]]; then
                COMPREPLY=( $(compgen -W "${fonts}" -- "${cur}") )
            else
                COMPREPLY=( $(compgen -W "-p --profile -e --encrypt --no-encrypt -t --title --tags -n --note -i --interactive --keep -D --denoise --no-denoise -O --overlay --no-overlay --overlay-style --overlay-font --font-size --overlay-font-size --overlay-title --no-overlay-title -v --verbose ${profiles}" -- "${cur}") )
            fi
            ;;
        play|preview)
            local entries
            entries=$(vj list -q 2>/dev/null)
            COMPREPLY=( $(compgen -W "${entries}" -- "${cur}") )
            ;;
        delete|del|rm|remove)
            if [[ "${cur}" == -* ]]; then
                COMPREPLY=( $(compgen -W "-f --force" -- "${cur}") )
            else
                local entries
                entries=$(vj list -q 2>/dev/null)
                COMPREPLY=( $(compgen -W "${entries}" -- "${cur}") )
            fi
            ;;
        encrypt|decrypt)
            local entries
            entries="all $(vj list -q 2>/dev/null)"
            COMPREPLY=( $(compgen -W "${entries}" -- "${cur}") )
            ;;
        stats|stat|s)
            COMPREPLY=( $(compgen -W "-m --months -y --year" -- "${cur}") )
            ;;
        completions)
            COMPREPLY=( $(compgen -W "bash zsh fish powershell elvish install" -- "${cur}") )
            ;;
        list|ls)
            COMPREPLY=( $(compgen -W "-q --quiet" -- "${cur}") )
            ;;
    esac
}
complete -F _vj_completions vj
"#
}

pub fn get_zsh_completion() -> &'static str {
    r#"#compdef vj

_vj_entries() {
    local -a entries
    entries=(${(f)"$(vj list -q 2>/dev/null)"})
    _describe -t entries 'vj entries' entries
}

_vj() {
    local -a commands
    commands=(
        'record:Record a new video entry'
        'import:Import video files from inbox or path'
        'inbox-server:Start local mobile upload web server'
        'preview-inbox:Display details for an inbox video'
        'play:Play entry in mpv'
        'preview:Display metadata, note, and storyboard for an entry'
        'list:List all journal entries'
        'random:Play a random past entry'
        'encrypt:Encrypt entries with GPG AES-256'
        'decrypt:Decrypt entries to plaintext'
        'delete:Permanently delete entries'
        'stats:Display storage, streaks, and contribution heatmap'
        'profiles:List available compression profiles'
        'fonts:List recommended retro fonts for OSD'
        'config:Open configuration file in editor'
        'completions:Generate or install shell completion scripts'
    )

    _arguments -C \
        '1: :->command' \
        '*:: :->args' && return 0

    case $state in
        command)
            _describe -t commands 'vj command' commands
            ;;
        args)
            case $words[1] in
                record)
                    _arguments \
                        '(-p --profile)'{-p,--profile}'[Compression profile]:profile:(potato compact terry balanced hq)' \
                        '(-e --encrypt)'{-e,--encrypt}'[Encrypt with GPG AES-256]' \
                        '--no-encrypt[Save unencrypted]' \
                        '(-t --title)'{-t,--title}'[Title of entry]:title:' \
                        '--tags[Comma-separated tags]:tags:' \
                        '(-n --note)'{-n,--note}'[Open editor for note]' \
                        '(-i --interactive)'{-i,--interactive}'[Prompt for title and note]' \
                        '--wait[Encode in foreground]' \
                        '--no-bg[Encode in foreground]' \
                        '(-D --denoise)'{-D,--denoise}'[Enable microphone noise reduction (afftdn)]' \
                        '--no-denoise[Disable microphone noise reduction]' \
                        '(-O --overlay)'{-O,--overlay}'[Enable retro OSD overlay]' \
                        '--no-overlay[Disable retro OSD overlay]' \
                        '--overlay-style[Retro OSD color style]:style:(vhs_yellow camcorder_white green amber cyan)' \
                        '--overlay-font[Retro OSD font]:font:(vt323 silkscreen press_start_2p share_tech_mono)' \
                        '--font-size[Retro OSD font size in pixels]:fontsize:' \
                        '--overlay-font-size[Retro OSD font size in pixels]:fontsize:' \
                        '--overlay-title[Show title in overlay]' \
                        '--no-overlay-title[Do not show title in overlay]' \
                        '(-v --verbose)'{-v,--verbose}'[Verbose output]'
                    ;;
                import)
                    _arguments \
                        '(-p --profile)'{-p,--profile}'[Compression profile]:profile:(potato compact terry balanced hq)' \
                        '(-e --encrypt)'{-e,--encrypt}'[Encrypt with GPG AES-256]' \
                        '--no-encrypt[Save unencrypted]' \
                        '(-t --title)'{-t,--title}'[Title of entry]:title:' \
                        '--tags[Comma-separated tags]:tags:' \
                        '(-n --note)'{-n,--note}'[Open editor for note]' \
                        '(-i --interactive)'{-i,--interactive}'[Prompt for title and note]' \
                        '--keep[Keep raw file in inbox]' \
                        '(-D --denoise)'{-D,--denoise}'[Enable microphone noise reduction (afftdn)]' \
                        '--no-denoise[Disable microphone noise reduction]' \
                        '(-O --overlay)'{-O,--overlay}'[Enable retro OSD overlay]' \
                        '--no-overlay[Disable retro OSD overlay]' \
                        '--overlay-style[Retro OSD color style]:style:(vhs_yellow camcorder_white green amber cyan)' \
                        '--overlay-font[Retro OSD font]:font:(vt323 silkscreen press_start_2p share_tech_mono)' \
                        '--font-size[Retro OSD font size in pixels]:fontsize:' \
                        '--overlay-font-size[Retro OSD font size in pixels]:fontsize:' \
                        '--overlay-title[Show title in overlay]' \
                        '--no-overlay-title[Do not show title in overlay]' \
                        '(-v --verbose)'{-v,--verbose}'[Verbose output]' \
                        '*:file:_files'
                    ;;
                play|preview)
                    _arguments \
                        '1:entry:_vj_entries' \
                        '(-v --verbose)'{-v,--verbose}'[Verbose output]'
                    ;;
                delete)
                    _arguments \
                        '(-f --force)'{-f,--force}'[Skip confirmation]' \
                        '*:entry:_vj_entries'
                    ;;
                stats)
                    _arguments \
                        '(-m --months)'{-m,--months}'[Number of months in heatmap]:months:(3 6 12)' \
                        '(-y --year)'{-y,--year}'[Display full 12 months (1 year)]'
                    ;;
                encrypt|decrypt)
                    _arguments \
                        '1:entry:("all" $(vj list -q 2>/dev/null))'
                    ;;
                completions)
                    _arguments \
                        '1:shell:(bash zsh fish powershell elvish install)'
                    ;;
            esac
            ;;
    esac
}

_vj "$@"
"#
}

pub fn print_custom_completion(shell_name: &str, cmd: &mut Command) {
    match shell_name {
        "fish" => {
            let mut safe = SafeStdout;
            let _ = safe.write_all(get_fish_completion().as_bytes());
        }
        "bash" => {
            let mut safe = SafeStdout;
            let _ = safe.write_all(get_bash_completion().as_bytes());
        }
        "zsh" => {
            let mut safe = SafeStdout;
            let _ = safe.write_all(get_zsh_completion().as_bytes());
        }
        "powershell" => {
            let mut safe = SafeStdout;
            generate(Shell::PowerShell, cmd, "vj", &mut safe);
        }
        "elvish" => {
            let mut safe = SafeStdout;
            generate(Shell::Elvish, cmd, "vj", &mut safe);
        }
        _ => {}
    }
}

pub fn install_completions() -> Result<()> {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    println!("Installing shell completions with dynamic entry resolution...");

    // 1. Fish
    let fish_dir = home.join(".config/fish/completions");
    if let Ok(()) = fs::create_dir_all(&fish_dir) {
        let fish_file = fish_dir.join("vj.fish");
        if fs::write(&fish_file, get_fish_completion()).is_ok() {
            println!("[✓] Installed Fish completion to {}", fish_file.display());
        }
    }

    // 2. Bash
    let bash_dir = home.join(".local/share/bash-completion/completions");
    if let Ok(()) = fs::create_dir_all(&bash_dir) {
        let bash_file = bash_dir.join("vj");
        if fs::write(&bash_file, get_bash_completion()).is_ok() {
            println!("[✓] Installed Bash completion to {}", bash_file.display());
        }
    }

    // 3. Zsh
    let zsh_dir = home.join(".zfunc");
    if let Ok(()) = fs::create_dir_all(&zsh_dir) {
        let zsh_file = zsh_dir.join("_vj");
        if fs::write(&zsh_file, get_zsh_completion()).is_ok() {
            println!(
                "[✓] Installed Zsh completion to {} (ensure ~/.zfunc is in fpath in ~/.zshrc)",
                zsh_file.display()
            );
        }
    }

    Ok(())
}
