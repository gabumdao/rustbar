#!/bin/sh
set -eu

usage() {
    cat <<'EOF'
Usage: install-completions.sh [--bash|--zsh|--fish|--all] [--prefix DIR]

Options:
  --bash        Install Bash completion only
  --zsh         Install Zsh completion only
  --fish        Install Fish completion only
  --all         Install all supported completion files (default)
  --prefix DIR  Override the install prefix used for Bash and Zsh completions
  -h, --help    Show this help

Default targets:
  Bash: $XDG_DATA_HOME/bash-completion/completions or ~/.local/share/bash-completion/completions
  Zsh:  $XDG_DATA_HOME/zsh/site-functions or ~/.local/share/zsh/site-functions
  Fish: ~/.config/fish/completions
EOF
}

install_bash=true
install_zsh=true
install_fish=true
prefix=${XDG_DATA_HOME:-$HOME/.local/share}

while [ $# -gt 0 ]; do
    case "$1" in
        --bash)
            install_bash=true
            install_zsh=false
            install_fish=false
            ;;
        --zsh)
            install_bash=false
            install_zsh=true
            install_fish=false
            ;;
        --fish)
            install_bash=false
            install_zsh=false
            install_fish=true
            ;;
        --all)
            install_bash=true
            install_zsh=true
            install_fish=true
            ;;
        --prefix)
            shift
            [ $# -gt 0 ] || { printf 'Missing value for --prefix
' >&2; exit 2; }
            prefix=$1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            printf 'Unknown option: %s

' "$1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
completions_dir="$repo_root/completions"

if [ "$install_bash" = true ]; then
    bash_target="$prefix/bash-completion/completions"
    mkdir -p "$bash_target"
    cp "$completions_dir/rustbarctl.bash" "$bash_target/rustbarctl"
    printf 'Installed Bash completion to %s
' "$bash_target/rustbarctl"
fi

if [ "$install_zsh" = true ]; then
    zsh_target="$prefix/zsh/site-functions"
    mkdir -p "$zsh_target"
    cp "$completions_dir/_rustbarctl" "$zsh_target/_rustbarctl"
    printf 'Installed Zsh completion to %s
' "$zsh_target/_rustbarctl"
fi

if [ "$install_fish" = true ]; then
    fish_target="${XDG_CONFIG_HOME:-$HOME/.config}/fish/completions"
    mkdir -p "$fish_target"
    cp "$completions_dir/rustbarctl.fish" "$fish_target/rustbarctl.fish"
    printf 'Installed Fish completion to %s
' "$fish_target/rustbarctl.fish"
fi
