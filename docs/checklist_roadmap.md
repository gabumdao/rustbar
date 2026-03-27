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
| Hotplug add/remove | [x] | [ ] | User verified live Hyprland hotplug add/remove behavior on 2026-03-26. |
| Resolution / output change behavior | [x] | [ ] | User verified live Hyprland output-change and resolution-change behavior on 2026-03-26. |
| Scale-factor handling | [x] | [ ] | User verified live Hyprland scale-factor handling on 2026-03-26. |
| Transform handling | [x] | [ ] | User verified live Hyprland transform handling on 2026-03-26. |

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

Run these in order if hardware and compositor support are available:

1. Start `rustbar` with all currently connected outputs active.
2. Disconnect one non-primary output.
3. Confirm the corresponding rustbar surface disappears and no stale frame remains.
4. Reconnect that output.
5. Confirm the surface returns on the reconnected output without creating duplicates on existing outputs.
6. If your compositor allows late output enablement, start `rustbar` before enabling an output and confirm the surface appears once metadata arrives.
7. Change monitor arrangement or resolution and confirm the affected output redraws correctly.

Checks:

- [x] Disconnecting an output removes the corresponding surface
- [x] Reconnecting an output recreates the corresponding surface
- [x] Late output enablement creates exactly one surface when metadata arrives
- [x] Resolution or arrangement changes redraw the correct output
- [x] No duplicated surfaces appear
- [x] No stale geometry remains after the change

Record:

- Outputs tested: User-verified on Hyprland across the active monitor set, 2026-03-26
- Result: Pass
- Notes: Hotplug add/remove, late surface creation, and resolution-change redraw behavior were all reported as passing in the live Hyprland session.

### 7. Scale and transform verification

These checks validate the implemented output-aware buffer handling, not the old redraw fallback.

Recommended order:

1. Change scale on one output only, if your setup allows it.
2. Confirm the frame stays aligned to the bezel and does not blur, crop, or leave stale content.
3. Restore the original scale before moving to transform tests.
4. Change output rotation/transform on one output only.
5. Confirm the frame remains attached to the correct edges and uses correct dimensions after quarter-turn rotation.

Checks:

- [x] Scale change keeps the frame visible
- [x] Scale change keeps the frame aligned to the output edges
- [x] Scale change leaves no stale content
- [x] Transform change keeps the frame visible
- [x] Quarter-turn transform keeps dimensions correct
- [x] Transform change leaves no stale content

Record:

- Outputs tested: User-verified on Hyprland with live scale and transform changes, 2026-03-26
- Result: Pass
- Notes: Scale changes kept the frame aligned and visible without stale content, and transform changes kept the frame attached to the correct edges with correct quarter-turn dimensions.

## Sway Checklist

Suggested inspection commands:

```bash
swaymsg -t get_outputs
swaymsg -t get_tree
swaymsg -t get_version
```

### 1. Baseline startup

- [ ] Start `rustbar`
- [ ] Confirm one frame appears per enabled monitor
- [ ] Confirm no unexpected focus change occurs on launch
- [ ] Confirm pointer input passes through the frame when `click_through = true`

Record:

- Outputs tested: __________
- Result: Pass / Partial / Fail
- Notes: __________

### 2. Layer selection

Repeat with `layer.level = "background"`, `"bottom"`, `"top"`, and `"overlay"`.

- [ ] `background` behaves as expected
- [ ] `bottom` behaves as expected
- [ ] `top` behaves as expected
- [ ] `overlay` behaves as expected
- [ ] Invalid layer value falls back to `bottom`

Record:

- Outputs tested: __________
- Result: Pass / Partial / Fail
- Notes: __________

### 3. Exclusive zone and focus behavior

- [ ] `exclusive_zone = 0` behaves as expected
- [ ] Non-zero `exclusive_zone` behaves as expected
- [ ] `keyboard_interactivity = "none"` avoids focus capture
- [ ] Non-click-through mode behaves as expected if tested

Record:

- Outputs tested: __________
- Result: Pass / Partial / Fail
- Notes: __________

### 4. Fullscreen expectations

Current expectation: no compositor-specific fullscreen integration is implemented for Sway yet.

- [ ] Confirm that fullscreen does not use Hyprland-specific hide behavior
- [ ] Confirm whether the frame remains visible, is obscured naturally, or shows another compositor-specific behavior
- [ ] Record whether current generic layer behavior is acceptable or needs future compositor support

Record:

- Outputs tested: __________
- Result: Pass / Partial / Fail
- Notes: __________

### 5. Output lifecycle and reload behavior

- [ ] Per-monitor disable on reload removes the corresponding surface
- [ ] Re-enabling that monitor recreates the surface
- [ ] Hotplug add/remove behaves correctly if hardware allows
- [ ] Resolution or arrangement changes redraw the correct output
- [ ] No duplicated surfaces or stale geometry appear

Record:

- Outputs tested: __________
- Result: Pass / Partial / Fail
- Notes: __________

### 6. Scale and transform behavior

- [ ] Scale changes keep the frame visible
- [ ] Scale changes keep the frame aligned to the output edges
- [ ] Transform changes keep the frame visible
- [ ] Quarter-turn transforms keep dimensions correct if tested
- [ ] No stale content remains after scale or transform changes

Record:

- Outputs tested: __________
- Result: Pass / Partial / Fail
- Notes: __________

## Operator Procedure

Use this pass in the following order so results are comparable across runs:

1. Start `rustbar` with the shared test config.
2. Confirm baseline visibility and input behavior before changing any outputs.
3. Run hotplug and resolution-change checks before scale/transform checks.
4. Record results immediately in the `Notes` field for each section.
5. If a failure appears, capture whether it is visual-only, protocol-visible, or both.

Suggested launch command:

```bash
RUST_LOG=info cargo run
```

Suggested inspection commands during the pass:

For Hyprland:

```bash
hyprctl layers
hyprctl monitors all
```

For Sway:

```bash
swaymsg -t get_outputs
swaymsg -t get_tree
```

For each failed step, record:

- compositor name and date
- output names involved
- whether the issue reproduces after restarting `rustbar`
- whether the issue reproduces after disconnect/reconnect or only after runtime changes

## Status

- Current state: Ready to run manually
- Reason: the code and docs baseline are in place; remaining work is live compositor validation.
- Resume after: completing the next compositor-specific pass, with Sway as the current target.

## Results Summary

### Hyprland

- Pass / Partial / Fail: Pass
- Main findings: protocol verification confirmed per-output layer-surface creation, overlay requests, `bottom` fallback, namespace fallback, and input-region request paths; user also confirmed click-through, fullscreen hide/show, exclusive-zone behavior, namespace visibility, and per-monitor reload disable/re-enable in a live session.
- Follow-up issues: broader compositor coverage still remains, with Sway as the next verification target.

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


## Performance Checklist

### Scope

Use this pass to record runtime measurements after the new debug redraw instrumentation landed in `src/state.rs`.

Goals:

- Measure idle CPU behavior with rustbar running but visually stable
- Measure redraw cost from the debug redraw logs
- Compare single-output and multi-output redraw cost
- Record whether patterns materially change redraw time

Out of scope:

- Deep profiling with external profilers
- Image-quality review
- Compositor correctness checks already covered elsewhere in this checklist

### Preconditions

- Build succeeds
- A visible test config exists
- `RUST_LOG=debug` is available in the launch command
- At least one config variant exists for:
  - solid fill only
  - gradient fill
  - pattern-enabled bezel
- A CPU observation tool is available, for example `top`, `htop`, or `pidstat`

### Suggested Launch

```bash
RUST_LOG=debug cargo run
```

Or use the helper script:

```bash
scripts/perf_capture.sh
```

Example redraw log line:

```text
redraw output=DP-1 surface_id=123 logical=1920x1080 buffer=3840x2160 scale=2 transform=Normal transparent=false render_ms=3.421 upload_ms=1.287 committed=true
```

Use `render_ms` for draw cost, `upload_ms` for SHM/buffer cost, and compare `logical` versus `buffer` to understand scale or transform effects.

### 1. Idle CPU Baseline

1. Start rustbar with a stable config and no active monitor/layout changes.
2. Let the session sit idle for at least 30 seconds.
3. Observe rustbar CPU usage using your preferred tool.
4. Record whether CPU returns near idle after startup settles.

Checks:

- [ ] Idle CPU observed after startup settles
- [x] No repeated redraw spam appears in the debug logs while visually idle
- [x] No unexpected fullscreen polling or watcher churn is visible in the logs

Record:

- Compositor: Hyprland 0.49.0
- Outputs active: DP-1, HDMI-A-1
- CPU tool used: `scripts/perf_capture.sh` (`ps` sampling once per second)
- Idle CPU result: Fail
- Notes: CPU never returned near idle during a ~6 minute dev-profile capture. Sampled CPU ranged from 26.10% to 99.00%, averaging 56.33%, even though only 8 redraw timing lines were recorded.

### 2. Single-Output Redraw Cost

1. Use one enabled output if possible.
2. Start with a simple solid-fill config.
3. Trigger redraws by saving config changes such as `opacity`, `color`, or `gradient.mode`.
4. Record the `render_ms` and `upload_ms` values from the debug logs.

Checks:

- [ ] Solid-fill redraw timings recorded
- [ ] Gradient redraw timings recorded
- [ ] Redraw logs include logical size, buffer size, scale, and transform

Record:

- Compositor: Not isolated yet
- Output tested: Not isolated yet
- Solid-fill redraw range: Not measured separately in this capture
- Gradient redraw range: Not measured separately in this capture
- Notes: The first captured run stayed on a two-output Hyprland session, so this section remains open.

### 3. Pattern-Enabled Redraw Cost

1. Enable a representative SVG pattern config.
2. Trigger redraws with pattern-related config changes such as spacing, scale, or color.
3. Compare the resulting `render_ms` and `upload_ms` values against the solid-fill baseline.

Checks:

- [ ] Pattern redraw timings recorded
- [ ] Relative cost versus solid fill noted
- [ ] No unexpected redraw storm occurs during pattern reloads

Record:

- Pattern asset: Not measured yet
- Pattern mode: Not measured yet
- Pattern redraw range: Not measured yet
- Relative cost versus solid fill: Not measured yet
- Notes: The first capture established a baseline problem before pattern-specific benchmarking.

### 4. Multi-Output Redraw Cost

1. Run rustbar on multiple outputs.
2. Trigger a redraw that affects all outputs, such as a global config save.
3. Record per-output timing lines from the debug logs.
4. Note whether any output is materially slower due to scale, transform, or pattern usage.

Checks:

- [x] Per-output redraw timings recorded
- [x] Mixed scale or transform outputs noted if present
- [x] Slowest output identified if timings differ materially

Record:

- Compositor: Hyprland 0.49.0
- Outputs tested: DP-1, HDMI-A-1
- Per-output redraw summary: 8 redraws total across two 1920x1080 outputs at scale 1 / Normal transform. `render_ms` ranged from 11643.795 to 12651.987 and `upload_ms` ranged from 1.135 to 9.032.
- Slowest output: DP-1 (`render_ms=12651.987`)
- Notes: The problem appears render-bound rather than upload-bound. Both outputs showed multi-second render time despite simple scale/transform settings.

### 5. Performance Summary

After running the pass, summarize:

- [x] Idle CPU result written back into the roadmap
- [x] Redraw cost notes written back into the roadmap
- [x] Any obvious regressions or hotspots called out
- [x] Any follow-up instrumentation needs identified

Summary fields:

- Overall result: Partial
- Key findings: After the first pattern-render allocation fix, a second Hyprland capture improved redraw timing from roughly 11.6s-12.7s down to 4.14s-4.22s per redraw, and upload averaged 2.742ms. CPU still remained far above idle, ranging from 8.40% to 78.00% with a 60.58% average.
- Follow-up actions: Keep performance as a bugfix priority, isolate the remaining render hotspot(s), and rerun single-output and pattern-specific captures after the next optimization.
