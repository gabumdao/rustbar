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
- unsupported `pattern.mode` values
- unsupported `pattern.blend` values
- unsupported `inner_shadow.falloff` values
- `preset` usage, because preset application is not implemented yet
- enabled pattern layers without `pattern.source`
- enabled pattern layers whose resolved SVG asset does not exist
- opacity values outside `0.0..=1.0`
- non-positive `pattern.transform.scale`

Reload behavior:

- If reload fails to parse or deserialize the config, rustbar keeps the previous live config and logs a warning instead of partially applying broken state.
- Validation warnings do not block startup or reload; they explain where rustbar will clamp values or fall back to defaults.

Logging:

- Rustbar uses `env_logger`.
- Set `RUST_LOG=warn` to see warnings only.
- Set `RUST_LOG=info` to also see informational startup/config messages.

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
| `preset` | String | (None) | Reserved for future preset support. The key exists in the schema, but preset application logic is not implemented yet. |

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
- Fullscreen hiding is currently Hyprland-specific. The implementation hides the frame by drawing a transparent buffer, clearing the exclusive zone, and forcing the surface to be non-interactive while fullscreen is active.
- Runtime compositor verification is still pending; the current behavior is verified in code and unit tests, not yet in an automated compositor test harness.
- Invalid values in this section now produce validation warnings during config load so fallback behavior is easier to spot.

## Output and Monitor Behavior

- Rustbar creates one layer-shell surface per enabled output.
- Global config is applied first; `[monitors."<output-name>"]` overrides are then merged on top for the matching Wayland output name.
- Disabling a monitor override during live reload removes that output's active surface, and re-enabling it recreates the surface.
- Output scale is tracked per surface. When the compositor supports `wl_surface.set_buffer_scale`, rustbar renders into scaled buffers instead of reusing unscaled output dimensions.
- Output transform is tracked per surface. When the compositor supports `wl_surface.set_buffer_transform`, rustbar applies the output transform and swaps buffer dimensions for quarter-turn rotations before rendering.
- Hotplug, resolution-change, scale-change, and rotation-change behavior still need broader live compositor verification.

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
- Parsed SVG trees are cached by resolved path and file modification time, so repeated redraws do not re-read and reparse unchanged assets.
- Missing, unreadable, or invalid SVG assets currently log a warning once per asset state and then skip rendering until the file changes again.

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
