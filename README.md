# Rustbar

**Rustbar** is a lightweight, visually striking Wayland layer-shell application written in Rust. It functions as a decorative overlay that draws beautiful, highly customizable borders and frames around your screen. Built with `smithay-client-toolkit` and `tiny-skia`, Rustbar is perfect for creating immersive, CRT-inspired visual effects or adding a sleek bezel to your Wayland desktop setup without getting in your way.

## Features

- **Wayland Native**: Communicates natively via the `wlr-layer-shell` protocol to perfectly align and anchor to your screen edges.
- **Unobtrusive Z-Order Management**: Renders behind your panels (like Waybar) by default using the `bottom` layer.
- **Extreme Customizability**: Adjust the color, opacity, solid or gradient bezel fill, thickness, explicit per-side frame toggles, frame margins, frame offsets, and border radius.
- **CRT-Inspired Visual Depth**: Includes robust built-in rendering for inner shadows, highlights, and bezel ridges to give deep visual depth to your display edge.
- **Configurable Inner Shadow**: A dedicated `[inner_shadow]` section allows for a soft, adjustable shadow that follows the inner contour of the bezel.
- **Per-Monitor Configurations**: Override default behaviors for individual monitors using their Wayland output names (e.g., `DP-1`, `HDMI-A-1`).
- **User-Defined Presets**: Define reusable style presets in config and apply them globally or per monitor before explicit overrides.
- **Output-Aware Buffer Handling**: Rustbar now applies per-output buffer scale and buffer transform state so HiDPI outputs and rotated outputs can use correctly sized buffers when the compositor supports those Wayland surface features.
- **Live Reloading**: Automatically reloads configuration on-the-fly when it detects changes to the `rustbar.toml` file.
- **Configurable Layer Behavior**: The layer-shell surface can be placed on `background`, `bottom`, `top`, or `overlay`, with configurable namespace, exclusive zone, click-through, and keyboard interactivity.
- **Hyprland Fullscreen Handling**: On Hyprland, rustbar listens for monitor/fullscreen changes and hides the frame on fullscreen outputs by swapping in a transparent, non-interactive surface state.
- **SVG Pattern Layer**: Decorative SVG motifs can be repeated along the frame bezel with configurable scaling, rotation, spacing, jitter, and per-side motif sources for highly stylized visual themes.

## Installation

Ensure you have a recent Rust toolchain installed, as well as a compatible Wayland compositor (Hyprland, Sway, etc).

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/rustbar.git # Change this URL to match your Forgejo/GitHub repository
   cd rustbar
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. The compiled binary will be located in the `target/release/` directory.

## Configuration

Rustbar uses a straightforward TOML file for configuration. It resolves config in this order: `RUSTBAR_CONFIG`, `XDG_CONFIG_HOME/rustbar/rustbar.toml`, `~/.config/rustbar/rustbar.toml`, then a local `./rustbar.toml` fallback. Config files can also `include` other TOML files, and included files participate in live reload. Global settings are merged first, presets can provide reusable baseline styles, and per-monitor overrides are applied by Wayland output name.

### Example `rustbar.toml`

```toml
# --- Global Settings ---
thickness = 16
radius = 24
opacity = 0.8
color = "#ff5555" # Example: Red frame

# --- Visual Effects (Shadows and Highlights) ---
shadow_width = 16.0
shadow_color = "#000000CC"
highlight_color = "#FFFFFF33"

[inner_shadow]
enabled = true
size = 20
opacity = 0.3
softness = 1.0
falloff = "ease_out"

# --- Shell / Layer Behavior ---
[layer]
level = "bottom"
click_through = true
keyboard_interactivity = "none"
namespace = "rustbar-frame"
exclusive_zone = 0

# --- Per-Monitor Overrides ---
# To override settings for a specific monitor, specify its Wayland output name:
[monitors."DP-1"]
color = "#0000ff" # Blue frame specifically for DP-1
thickness_top = 10
thickness_bottom = 20

# --- Decorative SVG Patterns ---
[pattern]
enabled = true
source = "star.svg"      # Default source for any side without its own override
mode = "diagonal"
opacity = 0.2

[pattern.sources]
top = "top-star.svg"
bottom = "bottom-star.svg"

[pattern.layout]
angle = 45.0              # Placement direction for diagonal bands

[pattern.transform]
rotation = 0.0            # Extra motif rotation after placement direction is chosen

[pattern.repeat]
spacing = 30
jitter_rotation = 15.0
```

*Note: Colors can optionally include an alpha channel (e.g. `#000000CC`).*

Runtime notes: rustbar creates one layer-shell surface per enabled output. On compositors that support the relevant `wl_surface` features, it now applies output-specific buffer scale and buffer transform automatically. Live compositor verification for scale changes and rotated outputs is still pending.

Validation and logging notes:

- Rustbar now emits load-time warnings for unsupported enum-like config values, missing pattern sources, missing SVG assets, invalid opacity ranges, and unknown or invalid preset definitions.
- Base `opacity` multiplies any alpha already present in bezel or gradient colors, so `#RRGGBBAA` and `opacity` combine rather than overriding each other.
- If live reload fails, rustbar keeps the previous config and logs a warning with the resolved config path instead of applying a broken partial state.
- Pattern SVG parses are cached by path and modification time, and missing/invalid asset warnings are cached the same way so redraws do not spam the log.
- Use `RUST_LOG=warn` for warnings only, `RUST_LOG=info` for startup/config detail, or `RUST_LOG=debug` to log per-surface redraw timing with logical size, buffer size, scale, transform, and upload timing.
- Fatal startup failures now log clearer prerequisite errors for missing Wayland globals such as `wl_compositor`, `wl_shm`, and `zwlr_layer_shell_v1`.

## Compositor Support Status

Current verified status in this repository:

| Compositor | Status | Notes |
| :--- | :--- | :--- |
| Hyprland | Verified | Layer creation, click-through, exclusive zone, fullscreen hide/show, namespace visibility, per-monitor reload behavior, hotplug/output changes, scale handling, and transform handling were all verified manually on 2026-03-26. |
| Sway | Pending verification | Expected to support the generic layer-shell path, but no equivalent manual verification pass has been recorded yet. Hyprland-specific fullscreen hiding should not be assumed on Sway. |
| Other wlroots compositors | Needs verification | Generic layer-shell behavior may work, but there is no recorded support claim beyond code-level expectations. |

Support notes:

- The fullscreen hide/show integration currently has verified behavior only on Hyprland.
- Output-aware scale and transform handling are implemented in code and verified on Hyprland, but not yet on other compositors.
- For the current manual verification matrix and open items, see `docs/checklist_roadmap.md`. The Sway checklist in that file is now prepared to cover layer behavior, fullscreen expectations, output lifecycle, and scale/transform checks.

## Running

Execute the binary from within the directory containing your configuration file:

```bash
./target/release/rustbar
```

It's recommended to launch Rustbar via your compositor's autostart mechanism. For example, in Hyprland (`hyprland.conf`):

```conf
exec-once = cd ~/path/to/rustbar && ./target/release/rustbar
```

## Contributing

Contributions, feature requests, and bug reports are welcome on both GitHub and Forgejo! Open an issue or submit a pull request if you want to help make Rustbar even better.
