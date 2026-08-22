PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
FISH_COMPDIR ?= $(HOME)/.config/fish/completions
BASH_COMPDIR ?= $(HOME)/.local/share/bash-completion/completions
ZSH_COMPDIR ?= $(HOME)/.zfunc

.PHONY: all build install uninstall clean test

all: build

build:
	cargo build --release

test:
	cargo test

install: build
	@mkdir -p $(BINDIR)
	@install -m 755 target/release/vj $(BINDIR)/vj
	@echo "[✓] Installed binary to $(BINDIR)/vj"
	@mkdir -p $(FISH_COMPDIR) && ./target/release/vj completions fish > $(FISH_COMPDIR)/vj.fish
	@echo "[✓] Installed Fish completions to $(FISH_COMPDIR)/vj.fish"
	@mkdir -p $(BASH_COMPDIR) && ./target/release/vj completions bash > $(BASH_COMPDIR)/vj
	@echo "[✓] Installed Bash completions to $(BASH_COMPDIR)/vj"
	@mkdir -p $(ZSH_COMPDIR) && ./target/release/vj completions zsh > $(ZSH_COMPDIR)/_vj
	@echo "[✓] Installed Zsh completions to $(ZSH_COMPDIR)/_vj"
	@echo ""
	@echo "Installation successful! Ensure $(BINDIR) is in your PATH."

uninstall:
	@rm -f $(BINDIR)/vj
	@rm -f $(FISH_COMPDIR)/vj.fish
	@rm -f $(BASH_COMPDIR)/vj
	@rm -f $(ZSH_COMPDIR)/_vj
	@echo "[✓] Uninstalled vj binary and completions."

clean:
	cargo clean
