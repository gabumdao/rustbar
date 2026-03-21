# Rustbar Configuration Guide

This document describes all available configuration options for `rustbar.toml`.

## Global Settings

These settings apply to all monitors unless overridden in the `[monitors]` section.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `thickness` | Integer | `16` | Default thickness of the bar on all sides. |
| `thickness_top` | Integer | (None) | Specific thickness for the top edge. |
| `thickness_bottom` | Integer | (None) | Specific thickness for the bottom edge. |
| `thickness_left` | Integer | (None) | Specific thickness for the left edge. |
| `thickness_right` | Integer | (None) | Specific thickness for the right edge. |
| `radius` | Integer | `24` | Corner radius for the inner transparent area. |
| `opacity` | Float | `1.0` | Opacity of the bar bezel (0.0 to 1.0). |
| `color` | String | `"#000000"` | Hex color of the bar (e.g., `"#ff5555"`). Supports optional alpha `"#RRGGBBAA"`. |
| `preset` | String | (None) | Visual preset (e.g., `"crt"`). |

## Shadow and Highlight

These settings control the decorative effects on the bar bezel.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `shadow_width` | Float | `12.0` | Width of the inner shadow on the bezel. |
| `shadow_color` | String | `"#000000AA"` | Color and opacity of the shadow. |
| `highlight_width` | Float | `3.0` | Width of the outer bezel highlight. |
| `highlight_color` | String | `"#FFFFFF22"` | Color and opacity of the highlight. |

## Layer Configuration

The `[layer]` section controls how the bar interacts with the Wayland compositor.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `level` | String | `"bottom"` | Wayland layer: `"background"`, `"bottom"`, `"top"`, or `"overlay"`. |
| `exclusive_zone` | Integer | `0` | How much space (in pixels) the compositor should reserve for this bar. |
| `keyboard_interactivity`| String | `"none"` | Level of keyboard focus: `"none"`, `"on_demand"`, or `"exclusive"`. |
| `click_through` | Boolean | `true` | If true, clicks pass through the bar to windows underneath. |
| `namespace` | String | `"rustbar-frame"`| Namespace for the Wayland layer-shell surface. |

## Monitor-Specific Overrides

You can override any setting for a specific monitor using its name (find it via `hyprctl monitors`).

```toml
[monitors."DP-1"]
enabled = true          # Set to false to disable rustbar on this monitor
color = "#0000ff"
thickness = 10
shadow_width = 20.0

[monitors."HDMI-A-1"]
thickness_top = 32
thickness_bottom = 0    # No bar at the bottom
```

## Complete Example

```toml
thickness = 16
radius = 24
opacity = 0.9
color = "#1e1e2e"

shadow_width = 12.0
shadow_color = "#00000088"
highlight_width = 2.0
highlight_color = "#f5c2e733"

[layer]
level = "top"
exclusive_zone = 0
click_through = true
```
