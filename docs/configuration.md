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

## Inner Shadow

The `[inner_shadow]` section controls the shadow that fades inward from the bezel's inner edge, creating a sense of physical depth.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `enabled` | Boolean | `false` | Enable or disable the inner shadow. |
| `size` | Float | `10.0` | Inward distance of the shadow (in pixels). |
| `opacity` | Float | `0.5` | Max opacity of the shadow at the edge. |
| `color` | String | `"#000000"` | Hex color of the shadow. |
| `softness` | Float | `1.0` | Controls the falloff curve sharpness. |
| `falloff` | String | `"ease_out"` | Falloff style: `"ease_out"`, `"ease_in"`, or `"linear"`. |

## Layer Configuration

The `[layer]` section controls how the bar interacts with the Wayland compositor.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `level` | String | `"bottom"` | Wayland layer: `"background"`, `"bottom"`, `"top"`, or `"overlay"`. |
| `exclusive_zone` | Integer | `0` | How much space (in pixels) the compositor should reserve for this bar. |
| `keyboard_interactivity`| String | `"none"` | Level of keyboard focus: `"none"`, `"on_demand"`, or `"exclusive"`. |
| `click_through` | Boolean | `true` | If true, clicks pass through the bar to windows underneath. |
| `namespace` | String | `"rustbar-frame"`| Namespace for the Wayland layer-shell surface. |

## SVG Pattern Layer

The `[pattern]` section allows you to place repeating SVG motifs along the frame bezel.

### General

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `enabled` | Boolean | `false` | Enable or disable the pattern layer. |
| `source` | String | (None) | Filename of the SVG motif (e.g., `"star.svg"`). Resolved relative to `~/.config/rustbar/svg`. |
| `mode` | String | `"repeat"` | Placement mode: `"repeat"`, `"single"`, `"scatter"`, `"horizontal"`, `"vertical"`, `"diagonal"`, `"diagonal_repeat"`, `"diagonal_lines"`, `"checker"`, or `"diamond"`. `diagonal*` modes place motifs along real diagonal bands, not just rotated rows. |
| `opacity` | Float | `1.0` | Opacity of the pattern (0.0 to 1.0). |
| `color` | String | (None) | Tint color for the SVG (hex). Replaces all SVG colors. |
| `blend` | String | `"normal"` | Blend mode: `"normal"`, `"multiply"`, `"screen"`, `"overlay"`, `"darken"`, `"lighten"`. |
| `seed` | Integer | `0` | Random seed for deterministic jitter variation. |

### Transform

Sub-section: `[pattern.transform]`

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `scale` | Float | `1.0` | Global scale of the SVG motif. |
| `rotation` | Float | `0.0` | Extra motif rotation applied after the layout direction is chosen. This affects the SVG orientation, not the placement path. |
| `flip_x` | Boolean | `false` | Flip the motif horizontally. |
| `flip_y` | Boolean | `false` | Flip the motif vertically. |

### Mode Behavior

- `repeat`: draws a single repeated line through each enabled frame band.
- `single`: draws one motif centered in each enabled frame band.
- `scatter`: places motifs randomly within each enabled frame band.
- `horizontal`: draws rows aligned at `0°` before `pattern.transform.rotation` is added.
- `vertical`: draws rows aligned at `90°` before `pattern.transform.rotation` is added.
- `diagonal`, `diagonal_repeat`, `diagonal_lines`: place motifs along true diagonal lines that traverse each enabled bezel band. The line direction uses `pattern.layout.angle`.
- `checker`: draws an offset grid using the edge direction when `follow_edge_direction = true`.
- `diamond`: draws a staggered diagonal grid. Its motif orientation uses `pattern.layout.angle` plus any extra `pattern.transform.rotation`.

### Repeat Layout

Sub-section: `[pattern.repeat]`

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `spacing` | Float | `24.0` | Distance between repeated instances (in pixels). |
| `offset` | Float | `0.0` | Starting offset along each edge. |
| `jitter_spacing` | Float | `0.0` | Random variation in spacing. |
| `jitter_rotation`| Float | `0.0` | Random variation in rotation (in degrees). |
| `jitter_scale` | Float | `0.0` | Random variation in scale. |

### Side Controls

Sub-section: `[pattern.sides]`

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `top` | Boolean | `true` | Enable pattern on the top edge. |
| `bottom` | Boolean | `true` | Enable pattern on the bottom edge. |
| `left` | Boolean | `true` | Enable pattern on the left edge. |
| `right` | Boolean | `true` | Enable pattern on the right edge. |

### Corner Handling

Sub-section: `[pattern.corners]`

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `avoid_corners` | Boolean | `false` | If true, pattern stops before reaching corner areas. |
| `corner_padding`| Float | `12.0` | Padding distance from corners when `avoid_corners` is true. |

### Layout Controls

Sub-section: `[pattern.layout]`

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `follow_edge_direction` | Boolean | `true` | Rotate motifs to follow the edge direction when the selected mode uses edge orientation. |
| `clip_to_frame` | Boolean | `true` | Clip all pattern drawing so it stays inside the bezel area. |
| `prevent_overlap` | Boolean | `false` | If true, enforces extra spacing so repeated motifs do not overlap each other. |
| `overlap_padding` | Float | `2.0` | Extra separation added when `prevent_overlap` is enabled. |
| `angle` | Float | `45.0` | Direction angle in degrees for diagonal layouts. This controls placement direction for `diagonal`, `diagonal_repeat`, and `diagonal_lines`, and motif orientation for `diamond`. |

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
inner_shadow.size = 30  # Override only size for this monitor
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

[inner_shadow]
enabled = true
size = 20
opacity = 0.3
softness = 1.0
falloff = "ease_out"
```
