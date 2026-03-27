# Rustbar Configuration Guide

This document describes all available configuration options for `rustbar.toml`.

## Config Resolution and Reload

Rustbar resolves its main config file in this order:

1. `RUSTBAR_CONFIG`
2. `XDG_CONFIG_HOME/rustbar/rustbar.toml`
3. `~/.config/rustbar/rustbar.toml`
4. `./rustbar.toml`

Config files may use `include = "other.toml"` or `include = ["a.toml", "b.toml"]`. Included files are merged before deserialization, support variable expansion, and are watched for live reload together with the root config file.

## Validation and Logging

Rustbar currently validates configuration at load time and logs warnings for common silent-fallback cases.

Current warning categories include:

- unsupported `layer.level` values
- unsupported `layer.keyboard_interactivity` values
- unsupported `gradient.mode` values
- unsupported `pattern.mode` values
- unsupported `pattern.blend` values
- unsupported `inner_shadow.falloff` values
- enabled pattern layers without `pattern.source`
- enabled pattern layers whose resolved SVG asset does not exist
- opacity values outside `0.0..=1.0`
- non-positive `pattern.transform.scale`

Reload behavior:

- If reload fails to parse or deserialize the config, rustbar keeps the previous live config and logs a warning instead of partially applying broken state.
- Reload warnings now name the resolved config path so it is clearer which file failed.
- Validation warnings do not block startup or reload; they explain where rustbar will clamp values or fall back to defaults.

Logging:

- Rustbar uses `env_logger`.
- Set `RUST_LOG=warn` to see warnings only.
- Set `RUST_LOG=debug` to log per-surface redraw timing, including logical size, buffer size, scale, transform, render time, and SHM upload time.

Debug redraw log format:

```text
redraw output=DP-1 surface_id=123 logical=1920x1080 buffer=3840x2160 scale=2 transform=Normal transparent=false render_ms=3.421 upload_ms=1.287 committed=true
```

Field meanings:

- `logical`: compositor-reported logical size before output scale is applied
- `buffer`: actual SHM buffer size allocated for the draw
- `scale`: effective buffer scale accepted by the compositor surface
- `transform`: effective output transform applied to the buffer
- `render_ms`: time spent building the pixel buffer in rustbar
- `upload_ms`: time spent allocating/copying/attaching the SHM buffer
- `committed`: whether the buffer attach+commit path completed successfully
- Set `RUST_LOG=info` to also see informational startup/config messages.
- Successful startup and successful reload now log the resolved config path plus the number of loaded files, including includes.
- Fatal startup failures now log clearer messages for missing Wayland prerequisites such as `wl_compositor`, `wl_shm`, or `zwlr_layer_shell_v1`.

## Global Settings

These settings apply to all monitors unless overridden in the `[monitors]` section.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `thickness` | Integer | `16` | Default thickness of the bar on all sides. |
| `thickness_top` | Integer | (None) | Specific thickness for the top edge. |
| `thickness_bottom` | Integer | (None) | Specific thickness for the bottom edge. |
| `thickness_left` | Integer | (None) | Specific thickness for the left edge. |
| `thickness_right` | Integer | (None) | Specific thickness for the right edge. |
| `margin` | Integer | `0` | Base inset for the entire frame from the output edges. |
| `margin_top` | Integer | (None) | Specific top inset for the frame outer boundary. |
| `margin_bottom` | Integer | (None) | Specific bottom inset for the frame outer boundary. |
| `margin_left` | Integer | (None) | Specific left inset for the frame outer boundary. |
| `margin_right` | Integer | (None) | Specific right inset for the frame outer boundary. |
| `offset_x` | Integer | `0` | Horizontal translation applied after margins. Positive values move the frame right; negative values move it left. |
| `offset_y` | Integer | `0` | Vertical translation applied after margins. Positive values move the frame down; negative values move it up. |
| `radius` | Integer | `24` | Corner radius for the inner transparent area. |
| `opacity` | Float | `1.0` | Opacity multiplier for the bar bezel (0.0 to 1.0). This multiplies any alpha already present in `color`, gradient colors, highlight color, and shadow color. |
| `color` | String | `"#000000"` | Hex color of the bar (e.g., `"#ff5555"`). Supports optional alpha `"#RRGGBBAA"`; the final rendered alpha is `opacity * hex alpha`. |
| `preset` | String | (None) | Name of a preset from the top-level `[presets.<name>]` table. Presets are applied before explicit config values, so direct fields in the current scope still win. |
| `gradient.enabled` | Boolean | `false` | Enable a linear gradient fill for the bezel instead of a solid fill. |
| `gradient.mode` | String | `"vertical"` | Gradient direction: `"vertical"`, `"horizontal"`, `"diagonal_down"`, or `"diagonal_up"`. |
| `gradient.start_color` | String | `"#000000"` | Start color for the gradient fill. |
| `gradient.end_color` | String | `"#202020"` | End color for the gradient fill. |
| `frame_sides.top` | Boolean | `true` | Enable or disable the top base-frame segment explicitly. |
| `frame_sides.bottom` | Boolean | `true` | Enable or disable the bottom base-frame segment explicitly. |
| `frame_sides.left` | Boolean | `true` | Enable or disable the left base-frame segment explicitly. |
| `frame_sides.right` | Boolean | `true` | Enable or disable the right base-frame segment explicitly. |

## Frame Side Toggles

Rustbar now supports explicit base-frame side toggles through a top-level `[frame_sides]` table and per-monitor `[monitors."...".frame_sides]` tables.

Example:

```toml
[frame_sides]
top = true
bottom = true
left = true
right = false

[monitors."DP-1".frame_sides]
right = true
bottom = false
```

Rules:

- All frame sides default to `true`.
- Monitor-level `frame_sides` values override the global `frame_sides` values per side.
- `frame_sides` controls whether a side is drawn at all.
- `thickness_* = 0` still disables a side, so older configs continue to work.
- If a side is disabled with `frame_sides`, its thickness is treated as `0` for frame geometry and pattern clipping.

## Presets

Rustbar now supports user-defined presets through top-level preset tables.

Preset application order is:

1. Built-in defaults
2. Global preset from `preset = "..."`, if set
3. Explicit global config values
4. Monitor preset from `[monitors."..."] preset = "..."`, if set
5. Explicit monitor override values

Rules:

- Presets are defined as `[presets.<name>]` tables.
- Presets can include nested sections such as `[presets.crt.pattern]`, `[presets.crt.inner_shadow]`, or `[presets.crt.layer]`.
- A monitor can use a different preset from the global preset.
- If a preset name does not exist, rustbar logs a warning and keeps the explicit config values that were already present.
- Preset definitions are local config data, not built-in shipped themes.

Example:

```toml
preset = "crt"
color = "#1e1e2e"   # Explicit global values still override the preset

[presets.crt]
thickness = 22
radius = 30
shadow_width = 18.0

[presets.crt.inner_shadow]
enabled = true
size = 18
opacity = 0.35

[presets.crt.layer]
level = "top"

[monitors."DP-1"]
preset = "minimal"
opacity = 0.8

[presets.minimal]
thickness = 8
radius = 12

[presets.minimal.layer]
level = "bottom"
```

In that example, the global config uses `crt`, `color` still overrides the preset globally, and monitor `DP-1` switches to the `minimal` preset before applying its local `opacity`.

## Frame Geometry Notes

Rustbar's current frame geometry model is:

- The outer screen-edge boundary is rectangular and uses sharp corners. Margins and offsets now move that outer boundary inward or translate it within the output surface.
- The inner transparent cutout is the only rounded part of the base frame geometry.
- `radius` applies to that inner cutout, not to the outer display boundary.
- Per-side base-frame activation can now be controlled either by explicit `frame_sides` booleans or by setting a side thickness to `0`.

Practical behavior:

- Setting `thickness_top = 0`, `thickness_bottom = 0`, `thickness_left = 0`, or `thickness_right = 0` removes the bezel on that side.
- Setting `frame_sides.top = false`, `frame_sides.bottom = false`, `frame_sides.left = false`, or `frame_sides.right = false` disables that side even if it still has a non-zero thickness configured.
- `margin` insets the entire frame uniformly, while `margin_*` lets you inset specific edges differently.
- `offset_x` and `offset_y` translate the entire frame after margins are applied, which can intentionally create asymmetrical placement inside the output surface.
- Highlight strokes follow the sharp outer boundary, so outer corners stay sharp even when the inner cutout is rounded.
- Very large `radius` values are clamped by the inner rectangle size during rendering.
- Rounded inner-cutout edges are anti-aliased; current regression coverage checks for partial-alpha pixels along that curved boundary.
- If combined thickness values leave no inner rectangle, rustbar does not render a valid rounded cutout for that output geometry.

Current limitations:

- There is no separate outer-corner radius.
- Both `frame_sides` and zero-thickness side disabling are supported today; `frame_sides` is the clearer contributor-facing option for new configs.
- Margins and `offset_x` / `offset_y` are implemented, but separate outset/expansion controls are still not implemented.

## Gradient Fill

Rustbar now supports a first-pass linear bezel gradient through a top-level `[gradient]` table and per-monitor `[monitors."...".gradient]` overrides.

Example:

```toml
[gradient]
enabled = true
mode = "vertical"
start_color = "#3b4252"
end_color = "#0f111a"

[monitors."DP-1".gradient]
mode = "diagonal_down"
end_color = "#1a1b26"
```

Rules:

- `gradient.enabled = true` switches the base bezel fill from solid color to gradient fill.
- `opacity` still applies to both gradient colors, multiplying any alpha already present in `gradient.start_color` and `gradient.end_color`.
- Highlight, bezel shadow, inner shadow, and pattern rendering still draw on top of the gradient-filled bezel.
- Unsupported `gradient.mode` values fall back to `vertical` with a validation warning.
- `color` is still accepted and remains the fallback when gradients are disabled.

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

The `[layer]` section controls how the bar interacts with the Wayland compositor. These settings are applied when a surface is created, when config is reloaded, and when fullscreen handling changes the effective shell behavior.

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `level` | String | `"bottom"` | Wayland layer: `"background"`, `"bottom"`, `"top"`, or `"overlay"`. |
| `exclusive_zone` | Integer | `0` | How much space (in pixels) the compositor should reserve for this bar. When rustbar hides itself on a Hyprland fullscreen output, the effective exclusive zone is temporarily forced to `0`. |
| `keyboard_interactivity`| String | `"none"` | Level of keyboard focus: `"none"`, `"on_demand"`, or `"exclusive"`. Hidden fullscreen surfaces are always forced to `"none"`. |
| `click_through` | Boolean | `true` | If true, clicks pass through the bar to windows underneath. Hidden fullscreen surfaces are always forced to click-through. |
| `namespace` | String | `"rustbar-frame"`| Namespace for the Wayland layer-shell surface. Blank values currently fall back to `"rustbar-frame"`. |


### Layer Behavior Notes

- Invalid `layer.level` values currently fall back to `"bottom"`.
- Invalid `layer.keyboard_interactivity` values currently fall back to `"none"`.
- Default focus-avoidance behavior is `keyboard_interactivity = "none"`. On Hyprland, the verified fullscreen-hidden state also forces the surface to be non-interactive and click-through while fullscreen is active.
- Fullscreen hiding is currently Hyprland-specific. The implementation hides the frame by drawing a transparent buffer, clearing the exclusive zone, and forcing the surface to be non-interactive while fullscreen is active.
- Hyprland verification on 2026-03-26 confirmed namespace visibility in compositor tooling such as `hyprctl layers`.
- Runtime compositor verification is still incomplete overall; Hyprland shell behavior is manually verified, but equivalent coverage does not yet exist for Sway or other compositors.
- Invalid values in this section now produce validation warnings during config load so fallback behavior is easier to spot.

## Output and Monitor Behavior

- Rustbar creates one layer-shell surface per enabled output.
- Global config is applied first; `[monitors."<output-name>"]` overrides are then merged on top for the matching Wayland output name.
- Disabling a monitor override during live reload removes that output's active surface, and re-enabling it recreates the surface.
- Output scale is tracked per surface. When the compositor supports `wl_surface.set_buffer_scale`, rustbar renders into scaled buffers instead of reusing unscaled output dimensions.
- Output transform is tracked per surface. When the compositor supports `wl_surface.set_buffer_transform`, rustbar applies the output transform and swaps buffer dimensions for quarter-turn rotations before rendering.
- Hyprland verification on 2026-03-26 confirmed hotplug add/remove, resolution changes, scale changes, and transform changes. Broader live compositor verification still needs to be added for Sway and other compositors.

### Output Lifecycle

Rustbar's current output model is:

1. Detect a Wayland output.
2. Wait until output metadata is available.
3. Create one layer-shell surface for that output if it is enabled by config.
4. Apply global config plus any matching monitor override.
5. Draw using that output's current logical size, scale, and transform.

At runtime, rustbar currently reacts to output changes in these ways:

- Output removal destroys the corresponding active surface.
- Output updates trigger a reconcile path that refreshes the existing surface instead of creating duplicates.
- If an output was announced before its metadata was complete, rustbar can now create the surface later when the compositor finishes advertising that output.
- Config reload reevaluates which outputs are enabled, so a monitor can be disabled and re-enabled without restarting rustbar.

Current limits:

- Hotplug and output-change handling are implemented in code, but broader live compositor verification is still deferred.
- There is no documented guarantee yet for every compositor's ordering of output add, update, and remove events.
- Multi-monitor correctness is based on the current reconcile logic and manual Hyprland checks, not on automated integration coverage.

### Monitor Override Precedence

Rustbar applies monitor configuration in this order:

1. Built-in defaults
2. Global values from the root config
3. Matching `[monitors."<output-name>"]` overrides

Practical implications:

- If an override omits a field, the global value remains in effect.
- Setting `enabled = false` for a monitor suppresses that output's surface even if the global config is otherwise valid.
- A monitor override changes runtime behavior only for the matching Wayland output name, not for all outputs with similar geometry.

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

Pattern asset behavior:

- Pattern SVGs are resolved from the configured path, the XDG config SVG directory, the home config SVG directory, or the local working directory.
- Absolute paths are accepted directly; relative paths are resolved through the rustbar SVG search locations.
- Parsed SVG trees are cached by resolved path and file modification time, so repeated redraws do not re-read and reparse unchanged assets.
- Missing, unreadable, or invalid SVG assets currently log a warning once per asset state and then skip rendering until the file changes again.
- Pattern tinting replaces SVG colors at draw time; the original asset file is not modified.

### Transform

Sub-section: `[pattern.transform]`

| Option | Type | Default | Description |
| :--- | :--- | :--- | :--- |
| `scale` | Float | `1.0` | Global scale of the SVG motif. |
| `rotation` | Float | `0.0` | Extra motif rotation applied after the layout direction is chosen. This affects the SVG orientation, not the placement path. |
| `flip_x` | Boolean | `false` | Flip the motif horizontally. |
| `flip_y` | Boolean | `false` | Flip the motif vertically. |

### Pattern Behavior Notes

- `seed` controls deterministic jitter. The same config and seed produce stable spacing, rotation, and scale variation across redraws.
- `blend` controls how the pattern layer composites onto the bezel after the SVG is rendered. Unsupported values fall back to `normal` with a validation warning.
- `clip_to_frame = true` clears any pattern pixels outside the bezel ring, so motifs do not spill into the inner transparent cutout or beyond the frame. Current regression coverage now checks both inner-cutout clipping and outer-boundary clipping when margins inset the frame.
- `prevent_overlap = true` increases effective spacing and row/column steps so repeated motifs stay separated by at least the configured `overlap_padding`.
- `follow_edge_direction = true` rotates motif placement to follow edge direction for directional modes; diagonal families still use `pattern.layout.angle` for their traversal direction.
- `avoid_corners` and `corner_padding` currently trim top and bottom runs explicitly. Left and right behavior is still driven indirectly by the bezel band bounds rather than a fully symmetric corner model.

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

## Pattern Examples

### Simple Repeat Along All Sides

```toml
[pattern]
enabled = true
source = "star.svg"
mode = "repeat"
opacity = 0.18
color = "#f5e0dc"

[pattern.repeat]
spacing = 28
offset = 6

[pattern.layout]
clip_to_frame = true
```

Use this as the baseline pattern setup when you want a clean repeated motif with no jitter.

### Diagonal Bezel Sweep

```toml
[pattern]
enabled = true
source = "scanline.svg"
mode = "diagonal_repeat"
opacity = 0.12
blend = "screen"

[pattern.transform]
scale = 0.8
rotation = 0.0

[pattern.repeat]
spacing = 22

[pattern.layout]
angle = 35.0
follow_edge_direction = false
clip_to_frame = true
prevent_overlap = true
overlap_padding = 3
```

Use this when you want the pattern direction to be driven by a single diagonal angle instead of by each edge.

### Deterministic Scatter With Jitter

```toml
[pattern]
enabled = true
source = "dust.svg"
mode = "scatter"
opacity = 0.1
seed = 42

[pattern.transform]
scale = 0.7
rotation = 8

[pattern.repeat]
spacing = 26
jitter_spacing = 8
jitter_rotation = 18
jitter_scale = 0.15

[pattern.layout]
clip_to_frame = true
```

Use this for textured variation that stays stable across redraws because the seed is fixed.

### Checker Pattern With Corner Avoidance

```toml
[pattern]
enabled = true
source = "tile.svg"
mode = "checker"
opacity = 0.16
blend = "overlay"

[pattern.transform]
scale = 0.9

[pattern.corners]
avoid_corners = true
corner_padding = 18

[pattern.layout]
follow_edge_direction = true
prevent_overlap = true
overlap_padding = 4
```

Use this when you want a denser grid look but still want extra separation near the top and bottom corner regions.

### Side-Limited Diamond Pattern

```toml
[pattern]
enabled = true
source = "diamond.svg"
mode = "diamond"
opacity = 0.14
color = "#89b4fa"

[pattern.transform]
scale = 0.85
rotation = 12

[pattern.sides]
top = true
bottom = false
left = true
right = false

[pattern.layout]
angle = 45
follow_edge_direction = false
```

Use this when you want motifs only on selected sides without disabling the whole pattern layer.

## Pattern Visual Comparison Checklist

Use this checklist when adding future screenshots or visual regression references for the pattern system.

Capture at minimum:

- `repeat` with a simple single-color motif
- `scatter` with deterministic jitter enabled
- one diagonal-family mode such as `diagonal_repeat`
- one grid-family mode such as `checker` or `diamond`
- one example with `avoid_corners = true`
- one example with `prevent_overlap = true`
- one example with `blend = "overlay"` or `blend = "screen"`
- one example with side-limited pattern enablement

For each captured example, record:

- the exact config snippet used
- the SVG asset name
- output scale and transform state
- compositor name
- whether the screenshot is intended as a style example or a regression reference

Recommended comparison pairs:

- `repeat` vs `scatter`
- `diagonal_repeat` vs `checker`
- `prevent_overlap = false` vs `prevent_overlap = true`
- `avoid_corners = false` vs `avoid_corners = true`
- `follow_edge_direction = true` vs `follow_edge_direction = false` for directional modes

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
