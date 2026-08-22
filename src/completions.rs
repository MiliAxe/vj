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

pub fn print_completions(shell: Shell, cmd: &mut Command) {
    let mut safe = SafeStdout;
    generate(shell, cmd, "vj", &mut safe);
}

pub fn install_completions(cmd: &mut Command) -> Result<()> {
    let home = std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("."));
    println!("Installing shell completions...");

    // 1. Fish
    let fish_dir = home.join(".config/fish/completions");
    if let Ok(()) = fs::create_dir_all(&fish_dir) {
        let fish_file = fish_dir.join("vj.fish");
        if let Ok(mut f) = fs::File::create(&fish_file) {
            generate(Shell::Fish, cmd, "vj", &mut f);
            println!("[✓] Installed Fish completion to {}", fish_file.display());
        }
    }

    // 2. Bash
    let bash_dir = home.join(".local/share/bash-completion/completions");
    if let Ok(()) = fs::create_dir_all(&bash_dir) {
        let bash_file = bash_dir.join("vj");
        if let Ok(mut f) = fs::File::create(&bash_file) {
            generate(Shell::Bash, cmd, "vj", &mut f);
            println!("[✓] Installed Bash completion to {}", bash_file.display());
        }
    }

    // 3. Zsh
    let zsh_dir = home.join(".zfunc");
    if let Ok(()) = fs::create_dir_all(&zsh_dir) {
        let zsh_file = zsh_dir.join("_vj");
        if let Ok(mut f) = fs::File::create(&zsh_file) {
            generate(Shell::Zsh, cmd, "vj", &mut f);
            println!(
                "[✓] Installed Zsh completion to {} (ensure ~/.zfunc is in fpath in ~/.zshrc)",
                zsh_file.display()
            );
        }
    }

    Ok(())
}
