# Rustbar Runtime Verification Checklist

This checklist captures the deferred manual verification pass for hotplug and output-change behavior. Core shell and layering verification is already complete on Hyprland, and it remains deferred even though scale-factor and transform handling is now implemented in code.

## Scope

Deferred goals for the next manual compositor pass:

- Verify hotplug add/remove behavior in a live compositor session
- Verify resolution and output-metadata change behavior after the new `update_output` reconcile path
- Validate the finished scale-factor and transform implementation on a live compositor
- Record compositor-specific differences for the roadmap

Out of scope for this pass:

- Performance benchmarking
- Rendering/image-quality review
- Pattern-mode visual regression review
- Further rendering refactors beyond validating the implemented output-aware scale and transform handling

## Preconditions

- `cargo build` or `cargo build --release` succeeds
- A test configuration file is available with visible frame settings
- At least one launcher method is ready for repeated restarts of `rustbar`
- On Hyprland, `hyprctl` is available
- On Sway, `swaymsg` is available

## Completed Protocol Verification (2026-03-26)

The following checks were completed against the live Wayland session using `WAYLAND_DEBUG=1` and temporary config files. These are runtime protocol verifications, not visual compositor checks.

Verified findings:

- `zwlr_layer_shell_v1` is available and `rustbar` successfully created one layer surface per output.
- Explicit layer configuration was observed live: `overlay` config produced `get_layer_surface(..., 3, Some("verify-shell"))`, followed by `set_layer(3)`, `set_exclusive_zone(17)`, and `set_keyboard_interactivity(1)`.
- Blank namespace fallback was observed live: blank `namespace` config produced `Some("rustbar-frame")`.
- Invalid `layer.level` fallback was observed live: invalid layer config produced `get_layer_surface(..., 1, ...)` and `set_layer(1)`, which matches `bottom`.
- Invalid `keyboard_interactivity` fallback was observed live: invalid config produced `set_keyboard_interactivity(0)`, which matches `none`.
- Input-region behavior was observed live:
  - `click_through = true` created a region and applied it with `set_input_region(region)`.
  - `click_through = false` used `set_input_region(<anonymous>@0)`, matching a cleared input region request.

Still not verified by this protocol run alone:

- Sway behavior

These Hyprland items were later confirmed manually in a live compositor session on 2026-03-26:

- Visual click-through behavior
- Visual fullscreen hide/show behavior
- Exclusive-zone behavior as experienced in the compositor layout
- Hyprland namespace visibility in compositor tooling
- Per-monitor disable/re-enable on reload

Additional code changes completed on 2026-03-26 before this verification pass:

- `update_output` now reconciles existing outputs instead of ignoring output changes.
- Outputs that were advertised before their metadata was ready can now create their surface later when info arrives.
- `scale_factor_changed`, `transform_changed`, and the main draw path now apply `set_buffer_scale` / `set_buffer_transform` and allocate buffers using output-aware dimensions.

## Test Matrix

| Area | Hyprland | Sway | Notes |
| :--- | :---: | :---: | :--- |
| Layer creation and visibility | [x] | [ ] | Protocol trace confirmed one layer surface per output on Hyprland, and user confirmed visibility in the live session. |
| Default `bottom` layer behavior | [ ] | [ ] | Frame stays behind top-layer panels where expected |
| Alternate layer selection | [x] | [ ] | `overlay` and invalid-value-to-`bottom` fallback were confirmed in the live protocol trace on Hyprland. |
| Click-through behavior | [x] | [ ] | User confirmed click-through works in a live Hyprland session. |
| Keyboard focus avoidance | [x] | [ ] | No focus issues were reported during the verified Hyprland session. |
| Exclusive zone behavior | [x] | [ ] | User confirmed exclusive-zone behavior in a live Hyprland session. |
| Namespace / compositor inspection | [x] | [ ] | Protocol trace confirmed configured namespace and blank fallback; user also confirmed namespace visibility in Hyprland tooling. |
| Fullscreen hide/show behavior | [x] | [ ] | User confirmed fullscreen hide/show behavior in a live Hyprland session. |
| Per-monitor disable on reload | [x] | [ ] | User confirmed disable/re-enable behavior on reload in a live Hyprland session. |
| Hotplug add/remove | [ ] | [ ] | Code path was tightened on 2026-03-26; still needs live compositor verification. |
| Resolution / output change behavior | [ ] | [ ] | `update_output` now reconciles and redraws, but this has not been verified live yet. |
| Scale-factor handling | [ ] | [ ] | Implemented in code with output-aware buffer scaling; still needs live compositor verification. |
| Transform handling | [ ] | [ ] | Implemented in code with output-aware buffer transforms and quarter-turn dimension swapping; still needs live compositor verification. |

## Shared Test Config

Use a clearly visible frame and explicit layer config during testing.

```toml
thickness = 18
radius = 28
opacity = 0.85
color = "#ff6b35"

[layer]
level = "bottom"
click_through = true
keyboard_interactivity = "none"
exclusive_zone = 0
namespace = "rustbar-frame"
```

For alternate-layer checks, change only `layer.level` between runs.

## Hyprland Checklist

### 1. Baseline startup

- [x] Start `rustbar`
- [x] Confirm one frame appears per enabled monitor
- [x] Confirm no unexpected focus change occurs on launch
- [x] Confirm pointer input passes through the frame when `click_through = true`

Record:

- Visible on all expected outputs: [x]
- Startup focus issue observed: [ ]
- Click-through works: [x]
- Notes: User-verified on Hyprland, 2026-03-26.

### 2. Layer selection

Repeat with `layer.level = "background"`, `"bottom"`, `"top"`, and `"overlay"`.

- [ ] `background` behaves as expected
- [ ] `bottom` behaves as expected
- [ ] `top` behaves as expected
- [ ] `overlay` behaves as expected
- [ ] Invalid layer value falls back to `bottom`

Notes: __________

### 3. Namespace / inspection tooling

Suggested commands:

```bash
hyprctl layers
hyprctl monitors
```

- [x] Surface is visible in layer inspection output
- [x] Namespace appears as configured or is otherwise attributable to `rustbar`
- [ ] Blank namespace fallback was checked if desired

Notes: Namespace visibility confirmed in Hyprland tooling on 2026-03-26.

### 4. Fullscreen behavior

Use a fullscreen-capable application on each monitor.

- [x] Enter fullscreen on monitor A and confirm the frame disappears on that monitor
- [x] Confirm the frame remains visible on other non-fullscreen monitors
- [x] Exit fullscreen and confirm the frame returns
- [x] Repeat on each monitor
- [x] Confirm fullscreen-hidden surface does not intercept pointer input
- [x] If `exclusive_zone != 0`, confirm fullscreen temporarily clears reserved-space behavior

Notes: Fullscreen hide/show and exclusive-zone behavior confirmed by user on Hyprland, 2026-03-26.

### 5. Config reload and per-monitor disable

Suggested flow:

1. Start with rustbar visible on multiple monitors.
2. Set `enabled = false` for one monitor in `[monitors."..."]`.
3. Save the config and allow live reload.

- [x] Disabled monitor surface is removed after reload
- [x] Remaining monitor surfaces stay alive and correct
- [x] Re-enabling the monitor restores the surface

Notes: Reload disable/re-enable behavior confirmed by user on Hyprland, 2026-03-26.

### 6. Hotplug and monitor changes

If hardware allows:

- [ ] Disconnect an output and confirm the corresponding surface is removed
- [ ] Reconnect the output and confirm the surface returns
- [ ] Start `rustbar` with an output that appears late and confirm the surface is created once output metadata arrives
- [ ] Change monitor arrangement or resolution and confirm the frame redraws correctly on the affected output
- [ ] Note whether any duplicated surfaces, stale geometry, or missing redraws appear

Notes: __________

### 7. Scale and transform fallback checks

These are fallback-behavior checks only, not full feature validation.

- [ ] Change output scale if your setup allows and confirm the frame redraws instead of disappearing or leaving stale content
- [ ] Change output rotation/transform if your setup allows and confirm the frame redraws instead of disappearing or leaving stale content
- [ ] Record whether the current redraw-only behavior is visually acceptable or clearly insufficient

Notes: __________

## Sway Checklist

### 1. Baseline startup

- [ ] Start `rustbar`
- [ ] Confirm one frame appears per enabled monitor
- [ ] Confirm no unexpected focus change occurs on launch
- [ ] Confirm pointer input passes through the frame when `click_through = true`

Notes: __________

### 2. Layer selection

- [ ] `background` behaves as expected
- [ ] `bottom` behaves as expected
- [ ] `top` behaves as expected
- [ ] `overlay` behaves as expected
- [ ] Invalid layer value falls back to `bottom`

Notes: __________

### 3. Exclusive zone and focus behavior

- [ ] `exclusive_zone = 0` behaves as expected
- [ ] Non-zero `exclusive_zone` behaves as expected
- [ ] `keyboard_interactivity = "none"` avoids focus capture
- [ ] Non-click-through mode behaves as expected if tested

Notes: __________

### 4. Fullscreen expectations

Current expectation: no compositor-specific fullscreen integration is implemented for Sway yet.

- [ ] Confirm that fullscreen does not use Hyprland-specific hide behavior
- [ ] Record whether current generic layer behavior is acceptable or needs future compositor support

Notes: __________

## Status

- Current state: Deferred
- Reason: manual compositor verification is still deferred even though the scale-factor and transform implementation work has landed.
- Resume after: documentation is updated or when you want to run the deferred live verification pass.

## Results Summary

### Hyprland

- Pass / Partial / Fail: Pass
- Main findings: protocol verification confirmed per-output layer-surface creation, overlay requests, `bottom` fallback, namespace fallback, and input-region request paths; user also confirmed click-through, fullscreen hide/show, exclusive-zone behavior, namespace visibility, and per-monitor reload disable/re-enable in a live session.
- Follow-up issues: live hotplug, resolution-change, scale verification, transform verification, and broader compositor coverage still remain.

### Sway

- Pass / Partial / Fail: __________
- Main findings: __________
- Follow-up issues: __________

## Roadmap Update Instructions

After running this checklist:

- Update `docs/rustbar-roadmap.md`
- Mark verified items in the Core Shell and Layering Behavior section
- Add compositor-specific findings to the notes fields
- Move any discovered problems into `Needs bugfix`, `Needs verification`, or `Needs refactor` as appropriate
