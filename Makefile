PREFIX ?= $(HOME)/.local
BINDIR ?= $(PREFIX)/bin
CARGO ?= cargo
INSTALL ?= install

.PHONY: build release install install-binaries install-completions uninstall-completions clean

build:
	$(CARGO) build

release:
	$(CARGO) build --release

install: install-binaries install-completions

install-binaries: release
	mkdir -p $(BINDIR)
	$(INSTALL) -m 0755 target/release/rustbar $(BINDIR)/rustbar
	$(INSTALL) -m 0755 target/release/rustbarctl $(BINDIR)/rustbarctl

install-completions:
	PREFIX=$(PREFIX) ./scripts/install-completions.sh --all

uninstall-completions:
	rm -f $(PREFIX)/bash-completion/completions/rustbarctl
	rm -f $(PREFIX)/zsh/site-functions/_rustbarctl
	rm -f $${XDG_CONFIG_HOME:-$(HOME)/.config}/fish/completions/rustbarctl.fish

clean:
	$(CARGO) clean
