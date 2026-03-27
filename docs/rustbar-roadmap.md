# Rustbar Product and Implementation Roadmap

## 1. Project Overview

`rustbar` is a Wayland layer-shell application that renders a decorative screen frame or bezel around one or more outputs. The current implementation focuses on:

- Full-screen per-output layer-shell surfaces
- Configurable bezel geometry and styling
- Optional inner shadow and highlight effects
- SVG motif repetition inside the bezel region
- Per-monitor configuration overrides
- Config reload on file changes
- Hyprland-specific fullscreen-aware layer adjustment

This document is the canonical reference for the current project state as verified in the repository on 2026-03-26. Status values below are based on the current code, tests, and existing documentation, not on prior discussion.

## 2. Current Status Summary

### What exists now

- Core Wayland layer-shell startup and per-output surface creation are implemented.
- One logical layer-shell surface is created per detected output.
- Decorative frame rendering is implemented with per-side thickness, inner rounded corners, outer highlight, bezel shadow, and optional inner shadow.
- SVG pattern rendering is implemented with multiple placement modes, transform controls, side enablement, corner avoidance, clipping, deterministic seeded jitter, and basic failure logging.
- TOML configuration loading is implemented with defaults, monitor overrides, include support, variable expansion, and config-file live reload.
- Output removal is handled, and new outputs can be added during sync/reload.
- Unit tests currently cover config resolution/include behavior, rendering paths, and shell/layer mapping helpers. A live Wayland protocol trace on 2026-03-26 also confirmed per-output layer-surface creation and several shell-policy requests on Hyprland.

### What is only partial or fragile

- Fullscreen handling now hides the frame on Hyprland outputs by swapping in a transparent buffer and clearing the exclusive zone; this was manually verified on Hyprland, but not yet on other compositors.
- Output updates now reconcile surfaces, apply output-aware buffer scale/transform state, and allocate scaled buffers, but live compositor verification is still incomplete.
- Layer-shell focus avoidance is improved through shared shell-policy application and passed the latest Hyprland verification pass, but no broader compositor-compatibility validation exists.
- Pattern and rendering features exist, but most are documented inconsistently relative to actual behavior.

### What is missing

- No gradients, texture layer, composition stack, IPC commands, animation, theme integration, or visual tooling. User-defined presets are now implemented, but shipped preset packs do not exist yet.
- No explicit validation/error-reporting layer for invalid config beyond parse failure and warning logs.
- No integration or compositor-level tests for multi-monitor, hotplug, fullscreen, click-through, or idle behavior.
- No documented release plan or production-readiness criteria. A basic compositor support matrix is now documented, but broader coverage is still incomplete.

## 3. Status Legend

- `Implemented`: Present in code and appears functional.
- `Partially implemented`: Present, but incomplete, narrow, or behaviorally limited.
- `Not implemented`: No meaningful implementation found.
- `Needs verification`: Code suggests support, but no test or runtime verification was found.
- `Needs documentation`: Implemented or partial, but repository docs are incomplete or misleading.
- `Needs refactor`: Works, but current structure is likely to hinder correctness or maintainability.
- `Needs bugfix`: Behavior is present but appears incorrect or materially weaker than expected.

## 4. Feature Audit Checklist

Legend for tracking columns:

- `Code`: verified in current codebase
- `Tested`: covered by tests in this repository
- `Docs`: reflected accurately in repository docs
- `Ready`: reasonable for contributor-facing roadmap purposes, not a production guarantee

### Core Shell and Layering Behavior

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Wayland layer-shell support | Implemented | [x] | [x] | [x] | [ ] | Uses `zwlr_layer_shell_v1` via `smithay-client-toolkit`; both protocol tracing and manual Hyprland verification confirmed per-output surface creation. | Extend compositor verification beyond Hyprland. |
| Correct default layer behavior | Implemented | [x] | [x] | [x] | [ ] | Default layer is `bottom`, and user verification confirmed expected Hyprland behavior. | Verify behavior on Sway. |
| Configurable `background` / `bottom` / `top` / `overlay` | Implemented | [x] | [x] | [x] | [ ] | String mapping exists with `bottom` fallback for invalid values and now has unit coverage. | Add invalid-value validation and examples. |
| Stable namespace handling | Implemented | [x] | [x] | [x] | [ ] | Namespace is configurable, blank values fall back to `rustbar-frame`, protocol tracing confirmed value selection, and user confirmed visibility in Hyprland tooling. | Verify behavior on additional compositors. |
| Compatibility with Hyprland inspection tools | Implemented | [x] | [x] | [x] | [ ] | User confirmed namespace visibility in Hyprland tooling during manual verification, and the configuration guide now mentions that verified behavior. | Extend verification to at least one non-Hyprland compositor. |
| Click-through behavior | Implemented | [x] | [x] | [x] | [ ] | Protocol tracing confirmed the request path and user verified the visual behavior on Hyprland. | Verify behavior on additional compositors. |
| Focus avoidance | Partially implemented | [x] | [x] | [x] | [ ] | Default keyboard interactivity is `none`, fullscreen-hidden surfaces are forced non-interactive, no focus issues were reported in the Hyprland pass, and the config guide now documents the verified behavior and compositor limits. | Extend verification beyond Hyprland. |
| Exclusive zone handling | Implemented | [x] | [x] | [x] | [ ] | Config value is applied to layer surfaces, fullscreen-hidden surfaces clear the exclusive zone, and user verified the behavior on Hyprland. | Verify behavior on additional compositors. |
| Fullscreen interaction behavior | Partially implemented | [x] | [x] | [x] | [ ] | Hyprland-only path hides the frame by rendering a transparent buffer and clearing the exclusive zone on fullscreen outputs; user verified hide/show behavior live. | Extend behavior or verification beyond Hyprland. |

Checklist:

- [x] Verified layer-shell creation in code
- [x] Verified layer-shell creation in a live Wayland protocol trace
- [x] Verified layer-shell creation visually in a Hyprland session
- [x] Verified namespace value and fallback in a live Wayland protocol trace
- [x] Verified namespace visibility in Hyprland tooling
- [x] Verified click-through request paths in a live Wayland protocol trace
- [x] Verified click-through visually in a compositor session
- [x] Verified fullscreen hiding logic in code
- [x] Verified fullscreen behavior in a compositor session
- [x] Documented current compositor-specific behavior and limits

### Multi-Monitor and Output Management

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| One logical surface per output | Implemented | [x] | [ ] | [x] | [ ] | `new_output` creates a dedicated surface per output, and `update_output` now avoids duplicate surface creation while reconciling delayed output metadata. | Add live verification notes for hotplug and late-arriving output metadata. |
| Output-specific configuration overrides | Implemented | [x] | [ ] | [x] | [ ] | `for_monitor` merges global and monitor-specific settings. | Add tests for override edge cases. |
| Monitor hotplug handling | Implemented | [x] | [x] | [x] | [ ] | Output removal is handled; `update_output` now creates a surface when output info becomes available later and reapplies config/redraws on output changes. User verification on Hyprland confirmed hotplug add/remove behavior. | Extend verification to at least one non-Hyprland compositor. |
| Resolution changes | Implemented | [x] | [x] | [x] | [ ] | Layer configure redraws on new size, output updates reconcile and refresh the affected surface, and the configuration guide now records the verified Hyprland behavior. | Extend verification to another compositor. |
| Output scale handling | Implemented | [x] | [x] | [x] | [ ] | Output scale is tracked per surface, `set_buffer_scale` is applied when supported, buffers are rendered at scaled dimensions, and user verification on Hyprland confirmed the live behavior. | Extend verification to at least one non-Hyprland compositor. |
| Rotation / transform handling | Implemented | [x] | [x] | [x] | [ ] | Output transform is tracked per surface, `set_buffer_transform` is applied when supported, quarter-turn transforms swap buffer dimensions before rendering, and user verification on Hyprland confirmed the live behavior. | Extend verification to at least one non-Hyprland compositor. |
| Stable behavior across multi-monitor sessions | Implemented, Needs broader verification | [x] | [x] | [x] | [ ] | Design supports multiple outputs, output updates reconcile existing surfaces safely, and the Hyprland manual pass now covers hotplug, output changes, scale changes, and transform changes. | Extend the same verification pass to another compositor and add integration coverage later. |
| Remove surfaces for monitors disabled by reload | Implemented | [x] | [x] | [x] | [ ] | `sync_outputs` now removes active surfaces whose monitor config is disabled before applying updates, and user verified disable/re-enable behavior on reload. | Add broader compositor verification notes if behavior differs outside Hyprland. |

Checklist:

- [x] Verified per-output surface creation in code
- [x] Tested hotplug add/remove
- [x] Tested resolution changes
- [x] Tested scale-factor changes with output-aware buffer scaling
- [x] Tested output rotation / transform changes with output-aware buffer transforms
- [x] Documented monitor override precedence

### Frame Geometry and Layout

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Frame thickness | Implemented | [x] | [x] | [x] | [ ] | Global and per-side thickness are supported, and the config guide now documents zero-thickness side behavior. | Add visual examples later. |
| Rounded inner corners | Implemented | [x] | [x] | [x] | [ ] | Inner transparent cutout uses rounded rectangle path. | Add screenshots/examples later. |
| Outer boundary handling | Implemented | [x] | [x] | [x] | [ ] | Outer edge remains sharp, highlight stroke uses sharp corners, and the config guide now states that only the inner cutout is rounded. | Add visual examples later. |
| Bezel ring clipping | Implemented, Needs verification | [x] | [x] | [ ] | [ ] | Pattern clipping is present; bezel rendering itself relies on carve-out path. | Add visual regression coverage. |
| Per-side activation | Implemented | [x] | [x] | [x] | [ ] | Base-frame side toggles now exist through `frame_sides`, and zero-thickness side disabling remains supported for compatibility. | Add visual examples later. |
| Corner padding | Partially implemented | [x] | [x] | [x] | [ ] | Only exists for pattern corner avoidance, not for core frame geometry. | Clarify scope in docs. |
| Offsets and margins | Implemented | [x] | [x] | [x] | [ ] | The frame outer boundary can now be inset with `margin` / `margin_*` and translated with `offset_x` / `offset_y`; geometry and pattern clipping follow the adjusted frame bounds. | Add visual examples and mixed-output verification later. |
| Geometry consistency across outputs | Needs verification | [x] | [ ] | [ ] | [ ] | Shared renderer is deterministic, but scale/transform gaps remain. | Test across mixed-monitor setups. |

Checklist:

- [x] Verified per-side thickness logic in code
- [x] Tested explicit base-frame side toggles
- [x] Verified rounded inner cutout logic in code
- [ ] Tested zero-thickness and extreme-radius edge cases visually
- [x] Documented current geometry constraints

### Base Rendering and Styling

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Solid fill | Implemented | [x] | [x] | [x] | [x] | Base bezel fill is a solid color. | None. |
| Opacity | Implemented | [x] | [x] | [x] | [ ] | Global opacity multiplies rendered alpha, including optional `#RRGGBBAA` color alpha and gradient stop alpha; regression coverage now checks both parsing and rendered output. | Add visual examples later. |
| Gradients | Implemented | [x] | [x] | [x] | [ ] | Linear bezel gradients now exist through the `gradient` config block with vertical, horizontal, and diagonal directions. | Add visual examples and compositor-side screenshots later. |
| Inner shadow | Implemented | [x] | [x] | [x] | [ ] | Separate inner-shadow pass supports falloff and softness. | Add examples and visual tests. |
| Texture support | Not implemented | [ ] | [ ] | [ ] | [ ] | No non-SVG texture or bitmap layer support. | Keep as post-1.0 enhancement. |
| Pattern layer support | Implemented | [x] | [x] | [x] | [ ] | SVG motif stamping is integrated. | Improve docs and runtime validation. |
| Rendering quality | Partially implemented | [x] | [x] | [ ] | [ ] | Anti-aliasing and bicubic resampling are used, and regression coverage now checks rounded-edge partial-alpha behavior, but there is still no profiling or broader image-quality baseline. | Add screenshot regression or benchmark process. |
| Clipping correctness | Partially implemented | [x] | [x] | [x] | [ ] | Pattern clip masking now has regression coverage for inner-cutout leaks and outer-boundary leaks when margins inset the frame, but broader visual validation across more shapes is still missing. | Add visual fixtures and compositor screenshots later. |
| Anti-aliasing quality | Partially implemented | [x] | [x] | [x] | [ ] | Anti-aliasing is enabled in major passes, and regression coverage now verifies partial-alpha pixels on the rounded inner cutout edge, but there is still no broader visual tuning baseline. | Evaluate on fractional scaling and thin geometry. |

Checklist:

- [x] Verified fill, highlight, bezel shadow, and inner shadow code paths
- [x] Tested linear gradient fill rendering
- [x] Verified pattern rendering path with tests
- [x] Tested clip masking against inner-cutout and outer-boundary leaks
- [x] Tested rounded-edge anti-aliasing on the inner cutout
- [x] Tested opacity interaction with hex alpha
- [ ] Benchmarked redraw cost
- [x] Documented current gradient behavior and limitations

### SVG Pattern System

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Asset loading | Implemented | [x] | [x] | [x] | [ ] | Reads SVG file, parses with `usvg`, renders via `resvg`. | Add better user-facing errors. |
| Default SVG directory handling | Implemented | [x] | [x] | [x] | [ ] | Resolves under `XDG_CONFIG_HOME/rustbar/svg` and `~/.config/rustbar/svg`. | Add examples and docs for include-based config layouts. |
| Relative path resolution from `~/.config/rustbar/svg` | Implemented | [x] | [x] | [x] | [ ] | Explicit resolver covers XDG and home config directories. | None. |
| Absolute path handling | Implemented | [x] | [ ] | [x] | [ ] | Absolute paths are accepted directly and are now documented explicitly in the configuration guide. | Add tests. |
| Repetition logic | Implemented | [x] | [x] | [x] | [ ] | Repeat, single, scatter/random, directional, checker, diamond, and diagonal families exist. | Add per-mode examples. |
| Spacing | Implemented | [x] | [x] | [x] | [ ] | `spacing` and `offset` are used in layout. | None. |
| Offset | Implemented | [x] | [x] | [x] | [ ] | Shared offset logic exists across modes. | None. |
| Rotation | Implemented | [x] | [x] | [x] | [ ] | Layout and transform rotation both exist. | Document mode-specific rotation semantics. |
| Scale | Implemented | [x] | [x] | [x] | [ ] | Global motif scale exists. | Document how extent-based scaling behaves. |
| Deterministic variation | Implemented | [x] | [x] | [x] | [ ] | Seeded RNG drives jitter. | Add tests for deterministic output stability if needed. |
| Per-side enable/disable | Implemented | [x] | [x] | [x] | [ ] | Pattern can be enabled per side. | None. |
| Corner handling | Partially implemented | [x] | [x] | [x] | [ ] | `avoid_corners` and padding exist for top/bottom trimming; left/right behavior is implicit through band bounds. | Add clearer docs and visual validation. |
| Diagonal mode | Implemented | [x] | [x] | [x] | [ ] | Real diagonal traversal is present. | Add examples and edge-case tests. |
| Path-follow support | Not implemented | [ ] | [ ] | [ ] | [ ] | No support for following rounded frame contours or arbitrary paths. | Defer to later release. |
| Failure handling for missing/invalid SVG files | Partially implemented | [x] | [x] | [x] | [ ] | Missing/read/parse failures are now cached by path and mtime so rustbar warns once per asset state instead of on every redraw, but there is still no surfaced UI/error summary. | Improve user-facing diagnostics. |
| SVG caching | Implemented | [x] | [x] | [x] | [ ] | Parsed SVG trees are now cached by resolved path plus modification time to avoid repeated reads and parses during redraws. | Verify it under live reload workflows. |

Checklist:

- [x] Verified SVG path resolution in code and tests
- [x] Verified pattern render path in tests
- [ ] Tested all placement modes visually
- [x] Documented all mode semantics with examples
- [x] Added SVG caching in code and regression coverage
- [x] Documented cache behavior and invalid-asset UX notes

### Configuration and User Experience

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| TOML configuration support | Implemented | [x] | [x] | [x] | [x] | Core config loading is present. | None. |
| Include support | Implemented | [x] | [x] | [x] | [ ] | `include` and variable expansion work and are now documented in the configuration guide. | Add more examples if needed. |
| Live reload / config reload | Implemented, Needs bugfix | [x] | [x] | [x] | [ ] | File watcher reloads all loaded config files, but visible updates are currently delayed by expensive serial redraws. In the latest Hyprland capture, `Reloaded config` logged immediately while the two monitor redraws landed about 4.2s apart, making simple color changes feel slow. | Reduce redraw cost and/or avoid synchronous full-surface serial redraw on reload. |
| Validation of invalid config | Partially implemented | [x] | [x] | [x] | [ ] | Parse errors still skip reload, but load-time validation warnings now flag silent fallback cases such as unsupported layer values, unsupported pattern settings, missing pattern sources, invalid opacity ranges, and unknown or invalid preset definitions. | Consider stronger schema validation later. |
| Fallback behavior | Implemented | [x] | [x] | [x] | [ ] | Config path, namespace fallback, and enum fallbacks are documented. | Add runtime verification notes where useful. |
| Default values | Implemented | [x] | [x] | [x] | [ ] | Defaults are declared in config structs. | Sync docs tightly to code. |
| Debug logging | Partially implemented | [x] | [x] | [x] | [ ] | `env_logger` is initialized, `RUST_LOG=debug` exposes per-surface redraw timing with output name, logical size, buffer size, scale, transform, upload timing, and commit status, and the repo now includes `scripts/perf_capture.sh` to capture those logs plus CPU samples into `output.log`. | Expand categories only if runtime debugging needs it. |
| Readable errors | Partially implemented | [x] | [x] | [x] | [ ] | Reload failures now name the resolved config path and state that rustbar is keeping the previous config, startup failures now report missing Wayland prerequisites more clearly, and config validation warnings are more specific. Errors are still primarily log-based. | Improve user-facing summaries further only if the current logs prove insufficient. |
| Documentation of configuration schema | Partially implemented | [x] | [ ] | [ ] | [ ] | Existing docs are incomplete and slightly ahead of or misaligned with implementation details. | Update docs after roadmap is merged. |

Checklist:

- [x] Verified path resolution order in code and tests
- [x] Verified include and variable expansion in tests
- [ ] Verified live reload end-to-end
- [x] Documented unsupported or fallback enum values
- [ ] Added examples for per-monitor layer and pattern overrides

### Performance and Runtime Behavior

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Idle CPU behavior | Needs bugfix | [x] | [x] | [x] | [ ] | First Hyprland captures on two 1920x1080 outputs never settled near idle in dev-profile runs. The latest capture ranged from 8.40% to 78.00% CPU with a 60.58% average, so the baseline issue is still unresolved even after the first pattern-render optimization. | Investigate the hot path, then repeat idle measurement in both dev and release builds. |
| Redraw behavior | Needs bugfix | [x] | [x] | [x] | [ ] | Redraw happens on configure and config reload; no frame callback loop is used. The first Hyprland performance capture recorded only 8 redraws at 11643.795ms to 12651.987ms each. After replacing the per-motif full-output temporary pixmap allocation in pattern rendering with a reusable motif-sized path, the next capture improved to 4138.053ms to 4215.834ms per redraw, which is materially better but still far too slow. | Investigate the next remaining render hotspot and rerun the performance capture after the next optimization. |
| Caching strategy | Partially implemented | [x] | [x] | [ ] | [ ] | SHM slot pools are reused per surface, SVG trees are cached by path plus mtime, and the renderer now keeps retained per-output frame, inner-shadow, and pattern layers with selective invalidation by config subsection. The cache architecture is in place, but performance still needs fresh measurement and likely more invalidation tuning. | Measure the retained-cache impact, then refine layer granularity if redraw cost is still too high. |
| Geometry recomputation rules | Partially implemented | [x] | [ ] | [ ] | [ ] | Geometry is recomputed on draw/configure, and scale/transform changes now feed the output-aware redraw path, but live guarantees are not verified yet. | Define explicit recomputation triggers and verify compositor behavior. |
| Multi-monitor performance | Needs bugfix | [x] | [x] | [x] | [ ] | Hyprland multi-output captures on DP-1 and HDMI-A-1 at 1920x1080 scale 1 / Normal transform remain in multi-second territory. The latest capture still ranged from 4138.053ms to 4215.834ms per redraw, with DP-1 peaking at 4215.834ms. | Fix the baseline render-cost issue, then repeat comparative multi-output measurements. |
| Startup behavior | Partially implemented | [x] | [ ] | [x] | [ ] | Startup now returns clearer fatal errors for missing Wayland prerequisites and logs which config path was loaded, but it still depends on runtime/manual validation rather than dedicated startup tests. | Add startup-focused tests only if this path keeps changing. |
| Runtime stability | Partially implemented | [x] | [x] | [ ] | [ ] | Unit tests pass, but runtime integration coverage is minimal. | Add compositor smoke tests and manual QA checklist. |

Checklist:

- [x] Verified slot-pool reuse in code
- [x] Measured idle CPU
- [ ] Measured redraw cost with patterns enabled
- [x] Added debug redraw timing instrumentation
- [x] Added a performance measurement checklist
- [x] Documented redraw timing log fields
- [x] Added a performance capture script
- [x] Captured first Hyprland performance baseline
- [ ] Documented performance expectations

### Future-Facing Features

| Feature | Status | Notes | Next action |
| :--- | :--- | :--- | :--- |
| Presets | Implemented | User-defined presets are resolved from top-level `[presets.<name>]` tables and can be applied globally or per monitor before explicit overrides. Shipped preset libraries and import/export still do not exist. | Add built-in preset packs only if they prove worth the complexity. |
| CRT mode | Not implemented | Mentioned conceptually only. | Treat as preset/theme work after 1.0. |
| Old TV bezel mode | Not implemented | No dedicated mode found. | Defer as preset pack. |
| Texture and pattern composition | Not implemented | Only SVG motif layer exists. | Design rendering stack first. |
| Side-specific motifs | Implemented, Needs documentation | Pattern side toggles exist, but not per-side asset selection. | Document current capability; defer distinct per-side motif sources. |
| Dedicated corner motifs | Not implemented | No corner-specific asset path or layout logic. | Consider for v1.5+. |
| Runtime IPC / commands | Not implemented | No control socket or command interface exists. | Revisit after config/live-reload hardening. |
| Theme integration | Not implemented | No compositor or desktop theme bridge. | Future only. |
| Pywal integration | Not implemented | No pywal code found. | Future only. |
| Advanced effects | Not implemented | Beyond current shadows/highlights/patterns, no effect stack exists. | Defer. |
| Animation support | Not implemented | No frame scheduling or animation state. | Defer until runtime budget is known. |
| Visual editor / tooling | Not implemented | No tooling in repo. | Future only. |
| Preset import/export | Not implemented | No preset format or packaging support. | Future only. |

Checklist:

- [ ] Remove or clearly mark unimplemented features in user-facing docs
- [ ] Define which future-facing items are product goals versus experiments

## 5. Documentation Gap Analysis

### High-priority gaps

- README and configuration docs must be kept aligned with the current Hyprland-specific fullscreen hide behavior and its verification limits.
- Equivalent compositor-specific verification notes still need to be added for Sway and any other non-Hyprland target.
- The configuration guide now includes a screenshot/comparison checklist for future pattern captures, but the actual screenshots are still missing.
- Visual compositor verification notes still need to be added once the remaining manual checks are run.

### Implemented but under-documented

- Detailed compositor-specific results for Sway and any other non-Hyprland verification target
- Click-through implementation details and compositor caveats beyond the currently documented Hyprland behavior
- Pattern mode semantics still lack captured screenshots
- Remaining pattern-system screenshot capture work
- Any additional non-default pattern workflows that still need example configs

### Likely misleading or incomplete docs

- Fullscreen behavior wording
- Production-readiness implied by feature lists
- Live compositor verification for scale and transform behavior is still missing
- Built-in preset packs still do not exist even though user-defined presets are now supported

## 6. Development Roadmap

### Phase 0: Audit Closure and Documentation Baseline

Goal: make the repository truthful before adding more surface area.

Scope:

- Align README and config docs with verified behavior
- Document unsupported, partial, and compositor-specific features
- Add a manual QA matrix for Hyprland and at least one non-Hyprland compositor
- Add tests for config merge edge cases, SVG path handling, and invalid config fallback behavior

Dependencies / blockers:

- None

Exit criteria:

- No user-facing doc claims behavior that the code does not implement
- The configuration schema is fully documented
- Basic contributor verification steps exist

### MVP: Stable Core Bezel Runtime

Goal: make `rustbar` reliable as a decorative per-output frame on mainstream Wayland compositors.

Priority features:

- Fix fullscreen behavior so it matches product intent
- Remove or reconcile surfaces for monitors disabled via config reload
- Verify and document click-through, focus avoidance, and exclusive-zone behavior
- Test hotplug and output-change behavior manually now that `update_output` reconciles surfaces
- Harden error handling around config reload and asset failures

Dependencies / blockers:

- Requires compositor-level manual testing

Implementation notes:

- Decide whether fullscreen behavior should hide, demote, or fully disable the surface
- Add an active-surface reconciliation step inside output sync

### v1.0: Correctness and Multi-Output Readiness

Goal: establish a trustworthy 1.0 core.

Priority features:

- Verify the newly implemented output scale-factor handling on live compositors
- Verify the newly implemented output transform/rotation handling on live compositors
- Add integration-level validation for multi-monitor sessions
- Validate the new SVG caching behavior under live reload and repeated redraw scenarios
- Improve config validation and user-facing diagnostics

Dependencies / blockers:

- Needs decisions on coordinate space and scaling strategy

Implementation notes:

- Treat scale and transform support as release blockers for a serious multi-monitor 1.0
- Introduce a small internal render/cache model before adding more visual features

### v1.1: Pattern System Polish

Goal: make the existing SVG pattern system easier to use and more predictable.

Priority features:

- Expand docs with per-mode examples
- Tighten corner handling semantics
- Add tests or fixtures for diagonal, checker, diamond, and overlap-prevention modes
- Improve invalid-asset reporting
- Consider per-side motif source or pattern presets if demand exists

Dependencies / blockers:

- Should build on v1.0 caching and validation work

### v1.5: Presets and Styling Workflow

Goal: reduce config friction and make repeatable themes practical.

Priority features:

- Validate whether built-in preset packs are still desirable now that user-defined presets exist
- Decide later whether to ship preset packs such as CRT-inspired, minimal, and ornamental styles
- Consider texture/pattern composition layers
- Consider dedicated corner motifs

Dependencies / blockers:

- Preset merge order now exists in code; future work would be about shipping preset content, not inventing the model

### v2.0: Advanced Runtime and Ecosystem Features

Goal: support dynamic workflows and richer customization beyond static config files.

Priority features:

- Runtime IPC or command interface
- Animation/effect scheduling
- Theme integration or pywal integration
- Preset import/export
- Visual editor or preview tooling

Dependencies / blockers:

- Requires stable cache/render architecture and measured runtime budget

Postpone until:

- Core compositor correctness, scale handling, and multi-monitor stability are solved

## 7. Open Questions and Technical Uncertainties

Selected rendering strategy:

- Use retained per-output layer caches with selective invalidation as the main performance fix.
- Keep the current renderer and dependencies; no new graphics library is selected at this stage.
- Use tiny-skia masks and cached pixmaps for compositing, and reserve pattern-shader specialization as an optional follow-up optimization for simple repeat modes only.

- What is the intended fullscreen product behavior: hide entirely, switch layer, disable input only, or user-selectable policy?
- Should rustbar keep supporting both `frame_sides` and zero-thickness side disabling long-term, or eventually recommend one canonical style?
- Should presets remain purely user-defined, or should rustbar eventually ship named preset packs?
- Should non-Hyprland fullscreen handling exist, and if so, how should compositor support be detected?
- Should SVG patterns be cached as parsed trees, rasterized motifs, or both?
- How should scale-factor and transform changes alter geometry: logical-pixel preservation, physical-pixel preservation, or configurable behavior?

## 8. Recommended Immediate Next Steps

1. Rerun performance captures in both debug and release builds now that retained per-output layer caches are in place.
2. Verify that one file save now produces one config reload, simple TOML color edits stay fast in release mode, and watcher-noise logs are reduced.
3. If redraw time is still too high, split cache granularity further or specialize simple pattern modes using tiny-skia pattern shaders.
4. After performance is under control, resume broader compositor verification and keep geometry/output-lifecycle docs aligned with any behavior changes.

## 9. Active Verification Step

The Hyprland shell/layer verification pass is now complete across shell behavior, per-monitor reload behavior, hotplug/output-change behavior, and output-aware scale/transform handling. The per-output lifecycle and monitor-override precedence are documented in the configuration guide, and the checklist now records the passing Hyprland results. The first Hyprland performance captures exposed a severe runtime issue: redraws started in the 11.6s-12.7s range, then improved into the 3.0s-4.2s range after several allocation-focused optimizations, but reload-visible updates were still blocked behind expensive serial redraws. The retained per-output layer-cache refactor is now implemented for the main frame, inner-shadow, and pattern layers. The next immediate runtime work is to rerun the performance capture and verify whether selective invalidation materially reduces redraw time and TOML color-update latency before refining cache granularity further.

## 10. Contributor Tracking Checklist

### Immediate engineering tasks

- [x] README corrected to match actual fullscreen behavior
- [x] `docs/configuration.md` updated with include/variables support
- [x] `preset` marked as unimplemented or implemented properly
- [x] Reload removes disabled monitor surfaces
- [x] Fullscreen behavior changed from layer-downgrade workaround to transparent-hide behavior on Hyprland
- [x] Hotplug add/remove tested after `update_output` reconciliation fix
- [x] Click-through tested on at least Hyprland
- [x] Exclusive-zone behavior tested
- [x] Scale-factor fallback replaced with proper output-aware handling
- [x] Frame margins and offsets implemented
- [x] Explicit base-frame side toggles implemented
- [x] Transform/rotation fallback replaced with proper output-aware handling
- [x] SVG caching implemented
- [x] Invalid-config diagnostics improved
- [x] Startup and reload diagnostics improved
- [x] Linear bezel gradients implemented
- [x] Added clipping regression coverage for pattern masking
- [x] Added anti-aliasing regression coverage for the rounded inner cutout
- [x] Added opacity and hex-alpha regression coverage
- [x] Added debug redraw timing instrumentation
- [x] Added a performance capture script
- [x] Identified a per-motif full-output pixmap allocation hotspot in pattern rendering
- [x] Applied the first pattern-render allocation optimization
- [x] Reused the rounded inner path across frame shadow and clear passes
- [x] Removed one full-surface temporary pixmap from the inner-shadow pass
- [x] Avoided a full-surface pattern pixmap when frame clipping is disabled
- [x] Implemented retained per-output frame, inner-shadow, and pattern layer caches
- [x] Split retained frame caching into base-fill and decor layers for narrower invalidation on color edits
- [x] Added stateful debounce to coalesce duplicate config reload events
- [x] Corrected the debounce regression that blocked legitimate hot reloads
- [x] Changed watcher reconfiguration to diff old/new watched files instead of unwatching and rewatching everything on each reload
- [x] Reran performance capture after the first optimization
- [ ] Identify the next remaining render hotspot
- [ ] Reduce reload latency for simple TOML color changes
- [x] Added opacity and hex-alpha regression coverage
- [x] Added anti-aliasing regression coverage for the rounded inner cutout

### Validation tasks

- [x] Unit tests currently pass with `cargo test`
- [x] Manual QA checklist added to repository
- [ ] Visual regression strategy defined
- [x] Multi-compositor support matrix documented

### Documentation tasks

- [ ] Feature matrix added to README or linked from README
- [ ] Configuration guide aligned with current schema and behavior
- [ ] Pattern mode examples added
- [ ] Multi-monitor behavior documented
- [ ] Runtime logging and troubleshooting documented
