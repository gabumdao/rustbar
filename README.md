# Rustbar

**Rustbar** is a lightweight, visually striking Wayland layer-shell application written in Rust. It functions as a decorative overlay that draws beautiful, highly customizable borders and frames around your screen. Built with `smithay-client-toolkit` and `tiny-skia`, Rustbar is perfect for creating immersive, CRT-inspired visual effects or adding a sleek bezel to your Wayland desktop setup without getting in your way.

## Features

- **Wayland Native**: Communicates natively via the `wlr-layer-shell` protocol to perfectly align and anchor to your screen edges.
- **Unobtrusive Z-Order Management**: Renders behind your panels (like Waybar) by default using the `bottom` layer.
- **Extreme Customizability**: Adjust the color, opacity, thickness (independently configurable for top, bottom, left, and right sides), and border radius.
- **CRT-Inspired Visual Depth**: Includes robust built-in rendering for inner shadows, highlights, and bezel ridges to give deep visual depth to your display edge.
- **Configurable Inner Shadow**: A dedicated `[inner_shadow]` section allows for a soft, adjustable shadow that follows the inner contour of the bezel.
- **Per-Monitor Configurations**: Override default behaviors for individual monitors using their Wayland output names (e.g., `DP-1`, `HDMI-A-1`).
- **Live Reloading**: Automatically reloads configuration on-the-fly when it detects changes to the `rustbar.toml` file.
- **Hyprland IPC Integration**: Features a built-in event listener connecting directly to Hyprland's socket to intelligently and automatically hide the frame whenever a fullscreen application (such as games or video players) is focused.

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

Rustbar uses a straightforward TOML file for configuration. By default, it expects a file named `rustbar.toml` in your current working directory.

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

# --- Per-Monitor Overrides ---
# To override settings for a specific monitor, specify its Waysland output name:
[monitors."DP-1"]
color = "#0000ff" # Blue frame specifically for DP-1
thickness_top = 10
thickness_bottom = 20
```

*Note: Colors can optionally include an alpha channel (e.g. `#000000CC`).*

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
