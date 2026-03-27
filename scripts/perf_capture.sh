#!/bin/sh
set -eu

OUTPUT_LOG="output.log"
RELEASE=0

usage() {
    cat <<'USAGE'
Usage: scripts/perf_capture.sh [--release] [-o output.log]

Starts rustbar with RUST_LOG=debug, captures runtime/debug output, samples CPU
usage once per second, and appends a summary when the process exits or is
interrupted.

Examples:
  scripts/perf_capture.sh
  scripts/perf_capture.sh --release
  scripts/perf_capture.sh -o logs/output.log
USAGE
}

while [ $# -gt 0 ]; do
    case "$1" in
        --release)
            RELEASE=1
            ;;
        -o|--output)
            shift
            [ $# -gt 0 ] || {
                echo "missing value for --output" >&2
                exit 1
            }
            OUTPUT_LOG="$1"
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
    shift
done

command -v cargo >/dev/null 2>&1 || {
    echo "cargo is required but not found in PATH" >&2
    exit 1
}

mkdir -p "$(dirname "$OUTPUT_LOG")"
: > "$OUTPUT_LOG"

RUN_DESC="RUST_LOG=debug cargo run"
if [ "$RELEASE" -eq 1 ]; then
    RUN_DESC="RUST_LOG=debug cargo run --release"
fi

append_line() {
    printf '%s\n' "$1" >> "$OUTPUT_LOG"
}

append_section() {
    append_line ""
    append_line "## $1"
}

append_command_output() {
    label="$1"
    shift
    append_line ""
    append_line "### $label"
    append_line "\$ $*"
    if command_output=$("$@" 2>&1); then
        printf '%s\n' "$command_output" >> "$OUTPUT_LOG"
    else
        printf '%s\n' "$command_output" >> "$OUTPUT_LOG"
    fi
}

append_header() {
    append_line "# Rustbar Performance Capture"
    append_line "timestamp=$(date -Is)"
    append_line "cwd=$(pwd)"
    append_line "command=$RUN_DESC"
    append_line "wayland_display=${WAYLAND_DISPLAY:-}"
    append_line "xdg_current_desktop=${XDG_CURRENT_DESKTOP:-}"
    append_line "swaysock=${SWAYSOCK:-}"
    append_line "hyprland_instance_signature=${HYPRLAND_INSTANCE_SIGNATURE:-}"

    append_section "System"
    append_command_output "uname" uname -a

    if command -v xbps-query >/dev/null 2>&1; then
        append_command_output "xbps-query" xbps-query -s void
    fi

    if command -v git >/dev/null 2>&1; then
        append_section "Git"
        append_command_output "git rev-parse" git rev-parse --short HEAD
        append_command_output "git status" git status --short
    fi

    if command -v hyprctl >/dev/null 2>&1; then
        append_section "Hyprland"
        append_command_output "hyprctl version" hyprctl version
        append_command_output "hyprctl monitors all" hyprctl monitors all
        append_command_output "hyprctl layers" hyprctl layers
    fi

    if command -v swaymsg >/dev/null 2>&1; then
        append_section "Sway"
        append_command_output "swaymsg -t get_version" swaymsg -t get_version
        append_command_output "swaymsg -t get_outputs" swaymsg -t get_outputs
    fi

    append_section "Instructions"
    append_line "1. Keep this script running while rustbar is active."
    append_line "2. Trigger redraws by editing config values and saving."
    append_line "3. Press Ctrl-C when you are done collecting runtime data."
}

sample_cpu_loop() {
    while kill -0 "$CARGO_PID" 2>/dev/null; do
        rustbar_pid="$(pgrep -n -x rustbar 2>/dev/null || true)"
        if [ -n "$rustbar_pid" ]; then
            ps -p "$rustbar_pid" -o pid=,%cpu=,%mem=,rss=,etime=,comm= \
                | awk -v ts="$(date -Is)" '
                    NF {
                        gsub(/^[[:space:]]+|[[:space:]]+$/, "", $0)
                        printf("cpu_sample timestamp=%s pid=%s cpu=%s mem=%s rss_kb=%s etime=%s comm=%s\n", ts, $1, $2, $3, $4, $5, $6)
                    }
                ' >> "$OUTPUT_LOG"
        fi
        sleep 1
    done
}

append_summary() {
    append_section "Summary"
    append_line "finished=$(date -Is)"

    redraw_count="$(grep -c 'redraw output=' "$OUTPUT_LOG" || true)"
    append_line "redraw_lines=$redraw_count"

    outputs="$({
        grep 'redraw output=' "$OUTPUT_LOG" \
            | sed -n 's/.*output=\([^ ]*\).*/\1/p' \
            | sort -u \
            | tr '\n' ' '
    } || true)"
    append_line "redraw_outputs=${outputs:-}"

    awk '
        /redraw output=/ {
            for (i = 1; i <= NF; i++) {
                if ($i ~ /^render_ms=/) {
                    split($i, a, "=")
                    render = a[2] + 0
                    render_sum += render
                    if (render_count == 0 || render < render_min) render_min = render
                    if (render_count == 0 || render > render_max) render_max = render
                    render_count++
                }
                if ($i ~ /^upload_ms=/) {
                    split($i, a, "=")
                    upload = a[2] + 0
                    upload_sum += upload
                    if (upload_count == 0 || upload < upload_min) upload_min = upload
                    if (upload_count == 0 || upload > upload_max) upload_max = upload
                    upload_count++
                }
            }
        }
        END {
            if (render_count > 0) {
                printf("render_ms_min=%.3f\nrender_ms_max=%.3f\nrender_ms_avg=%.3f\n", render_min, render_max, render_sum / render_count)
            }
            if (upload_count > 0) {
                printf("upload_ms_min=%.3f\nupload_ms_max=%.3f\nupload_ms_avg=%.3f\n", upload_min, upload_max, upload_sum / upload_count)
            }
        }
    ' "$OUTPUT_LOG" >> "$OUTPUT_LOG"

    awk '
        /^cpu_sample / {
            for (i = 1; i <= NF; i++) {
                if ($i ~ /^cpu=/) {
                    split($i, a, "=")
                    cpu = a[2] + 0
                    cpu_sum += cpu
                    if (cpu_count == 0 || cpu < cpu_min) cpu_min = cpu
                    if (cpu_count == 0 || cpu > cpu_max) cpu_max = cpu
                    cpu_count++
                }
            }
        }
        END {
            if (cpu_count > 0) {
                printf("cpu_percent_min=%.2f\ncpu_percent_max=%.2f\ncpu_percent_avg=%.2f\n", cpu_min, cpu_max, cpu_sum / cpu_count)
            }
        }
    ' "$OUTPUT_LOG" >> "$OUTPUT_LOG"
}

stop_children() {
    if [ -n "${SAMPLER_PID:-}" ] && kill -0 "$SAMPLER_PID" 2>/dev/null; then
        kill "$SAMPLER_PID" 2>/dev/null || true
        wait "$SAMPLER_PID" 2>/dev/null || true
    fi
    if [ -n "${CARGO_PID:-}" ] && kill -0 "$CARGO_PID" 2>/dev/null; then
        kill "$CARGO_PID" 2>/dev/null || true
        wait "$CARGO_PID" 2>/dev/null || true
    fi
}

on_signal() {
    append_line ""
    append_line "signal=$(date -Is) interrupted=true"
    stop_children
}

trap on_signal INT TERM

append_header
append_section "Rustbar Runtime"
append_line "\$ $RUN_DESC"

echo "Writing runtime capture to $OUTPUT_LOG"
echo "Trigger redraws manually, then press Ctrl-C when done."

if [ "$RELEASE" -eq 1 ]; then
    env RUST_LOG=debug cargo run --release >> "$OUTPUT_LOG" 2>&1 &
else
    env RUST_LOG=debug cargo run >> "$OUTPUT_LOG" 2>&1 &
fi
CARGO_PID=$!

sample_cpu_loop &
SAMPLER_PID=$!

wait "$CARGO_PID" 2>/dev/null || true
stop_children
append_summary

echo "Capture complete: $OUTPUT_LOG"
