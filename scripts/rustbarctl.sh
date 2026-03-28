#!/bin/sh
set -eu

usage() {
    cat <<'EOF'
Usage: rustbarctl.sh <command> [output-name]

Commands:
  reload       Reload the active config
  refresh      Redraw all active surfaces
  fullscreen   Rerun the fullscreen-state check
  show         Force all rustbar surfaces visible, or one output if given
  hide         Force all rustbar surfaces hidden, or one output if given
  toggle       Toggle manual visibility for all surfaces, or one output if given
  status       Print a live rustbar status summary
  quit         Stop rustbar cleanly
  help         Show this help
EOF
}

command=${1:-help}
shift || true
output_name=${*:-}
case "$command" in
    reload|refresh|fullscreen|show|hide|toggle|status|quit)
        ;;
    help|-h|--help)
        usage
        exit 0
        ;;
    *)
        printf 'Unknown command: %s

' "$command" >&2
        usage >&2
        exit 2
        ;;
esac

runtime_dir=${XDG_RUNTIME_DIR:-}
if [ -z "$runtime_dir" ]; then
    printf 'XDG_RUNTIME_DIR is not set.
' >&2
    exit 1
fi

socket_path="$runtime_dir/rustbar.sock"
if [ ! -S "$socket_path" ]; then
    printf 'Rustbar control socket not found: %s
' "$socket_path" >&2
    exit 1
fi

request=$command
if [ -n "$output_name" ]; then
    request="$request $output_name"
fi

if command -v socat >/dev/null 2>&1; then
    printf '%s
' "$request" | socat - UNIX-CONNECT:"$socket_path"
elif command -v nc >/dev/null 2>&1; then
    printf '%s
' "$request" | nc -U "$socket_path"
else
    printf 'Need socat or netcat with Unix-socket support to talk to %s
' "$socket_path" >&2
    exit 1
fi
