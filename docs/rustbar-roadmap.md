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

This document is the canonical reference for the current project state as verified in the repository on 2026-03-24. Status values below are based on the current code, tests, and existing documentation, not on prior discussion.

## 2. Current Status Summary

### What exists now

- Core Wayland layer-shell startup and per-output surface creation are implemented.
- One logical layer-shell surface is created per detected output.
- Decorative frame rendering is implemented with per-side thickness, inner rounded corners, outer highlight, bezel shadow, and optional inner shadow.
- SVG pattern rendering is implemented with multiple placement modes, transform controls, side enablement, corner avoidance, clipping, deterministic seeded jitter, and basic failure logging.
- TOML configuration loading is implemented with defaults, monitor overrides, include support, variable expansion, and config-file live reload.
- Output removal is handled, and new outputs can be added during sync/reload.
- Unit tests currently cover config resolution/include behavior and rendering paths.

### What is only partial or fragile

- Fullscreen handling is Hyprland-specific and currently downgrades fullscreen outputs to `bottom`, not true hide/disable behavior.
- Output updates such as scale-factor changes, transform changes, and geometry changes are not handled explicitly.
- Existing surfaces are updated on reload, but disabled monitors are not removed if they were already created.
- Layer-shell focus avoidance is mostly achieved through defaults, but no broader compositor-compatibility validation exists.
- Pattern and rendering features exist, but most are documented inconsistently relative to actual behavior.

### What is missing

- No gradients, texture layer, composition stack, presets implementation, IPC commands, animation, theme integration, or visual tooling.
- No explicit validation/error-reporting layer for invalid config beyond parse failure and warning logs.
- No integration or compositor-level tests for multi-monitor, hotplug, fullscreen, click-through, or idle behavior.
- No documented release plan, support matrix, or production-readiness criteria.

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
| Wayland layer-shell support | Implemented | [x] | [ ] | [x] | [ ] | Uses `zwlr_layer_shell_v1` via `smithay-client-toolkit`; surface created per output. | Add compositor-level smoke testing. |
| Correct default layer behavior | Implemented | [x] | [ ] | [x] | [ ] | Default layer is `bottom`. | Verify behavior on Hyprland and Sway. |
| Configurable `background` / `bottom` / `top` / `overlay` | Implemented | [x] | [ ] | [x] | [ ] | String mapping exists with `bottom` fallback for invalid values. | Add invalid-value validation and examples. |
| Stable namespace handling | Implemented, Needs verification | [x] | [ ] | [x] | [ ] | Namespace is configurable and passed into layer-surface creation. | Verify compositor/tooling visibility, especially Hyprland inspectors. |
| Compatibility with Hyprland inspection tools | Needs verification | [ ] | [ ] | [ ] | [ ] | Namespace support suggests compatibility, but no explicit validation or docs. | Test with `hyprctl layers` or equivalent tooling and document findings. |
| Click-through behavior | Implemented, Needs verification | [x] | [ ] | [x] | [ ] | Input region is cleared when `click_through = true`. | Add compositor-level tests and document caveats. |
| Focus avoidance | Partially implemented | [x] | [ ] | [ ] | [ ] | Default keyboard interactivity is `none`; no deeper focus policy handling. | Verify behavior across compositors and document limits. |
| Exclusive zone handling | Implemented, Needs verification | [x] | [ ] | [x] | [ ] | Config value is applied to layer surfaces. | Test reserved-space behavior on target compositors. |
| Fullscreen interaction behavior | Partially implemented, Needs bugfix | [x] | [ ] | [x] | [ ] | Hyprland-only path forces fullscreen outputs to `bottom`; README says it hides the frame, which is not what code does. | Decide intended behavior and implement hide/demote logic correctly. |

Checklist:

- [x] Verified layer-shell creation in code
- [ ] Verified click-through in a compositor session
- [ ] Verified fullscreen behavior matches docs
- [ ] Documented compositor-specific behavior and limits

### Multi-Monitor and Output Management

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| One logical surface per output | Implemented | [x] | [ ] | [ ] | [ ] | `new_output` creates a dedicated surface per output. | Document lifecycle clearly. |
| Output-specific configuration overrides | Implemented | [x] | [ ] | [x] | [ ] | `for_monitor` merges global and monitor-specific settings. | Add tests for override edge cases. |
| Monitor hotplug handling | Partially implemented | [x] | [ ] | [ ] | [ ] | Output removal is handled; new outputs are added during initial scan and sync. | Test true hotplug events and disabled-monitor transitions. |
| Resolution changes | Needs verification | [ ] | [ ] | [ ] | [ ] | Reconfigure path redraws on new size, but no explicit output update handling. | Run real compositor tests and document guarantees. |
| Output scale handling | Not implemented | [x] | [ ] | [ ] | [ ] | `scale_factor_changed` is empty. | Implement scale-aware buffer/render strategy. |
| Rotation / transform handling | Not implemented | [x] | [ ] | [ ] | [ ] | `transform_changed` is empty. | Implement transform-aware geometry/render behavior. |
| Stable behavior across multi-monitor sessions | Partially implemented, Needs verification | [x] | [ ] | [ ] | [ ] | Design supports multiple outputs, but no integration coverage exists. | Add manual test matrix and regression checklist. |
| Remove surfaces for monitors disabled by reload | Needs bugfix | [x] | [ ] | [ ] | [ ] | `sync_outputs` updates existing surfaces and creates new ones, but does not remove already-created surfaces that are now disabled. | Reconcile active surfaces against current config on reload. |

Checklist:

- [x] Verified per-output surface creation in code
- [ ] Tested hotplug add/remove
- [ ] Tested resolution changes
- [ ] Tested scale-factor changes
- [ ] Tested output rotation
- [ ] Documented monitor override precedence

### Frame Geometry and Layout

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Frame thickness | Implemented | [x] | [x] | [x] | [ ] | Global and per-side thickness are supported. | Add docs for zero-thickness side behavior. |
| Rounded inner corners | Implemented | [x] | [x] | [x] | [ ] | Inner transparent cutout uses rounded rectangle path. | Add screenshots/examples later. |
| Outer boundary handling | Implemented, Needs documentation | [x] | [x] | [ ] | [ ] | Outer edge remains sharp; highlight stroke uses sharp corners. | Document current outer-corner model explicitly. |
| Bezel ring clipping | Implemented, Needs verification | [x] | [x] | [ ] | [ ] | Pattern clipping is present; bezel rendering itself relies on carve-out path. | Add visual regression coverage. |
| Per-side activation | Partially implemented | [x] | [ ] | [ ] | [ ] | Thickness can be set to `0` per side, but there is no explicit side enable flag for the base frame. | Document zero-thickness strategy or add explicit side toggles. |
| Corner padding | Partially implemented | [x] | [x] | [x] | [ ] | Only exists for pattern corner avoidance, not for core frame geometry. | Clarify scope in docs. |
| Offsets and margins | Not implemented | [ ] | [ ] | [ ] | [ ] | No surface margin or frame inset/outset controls found. | Add layout model if needed. |
| Geometry consistency across outputs | Needs verification | [x] | [ ] | [ ] | [ ] | Shared renderer is deterministic, but scale/transform gaps remain. | Test across mixed-monitor setups. |

Checklist:

- [x] Verified per-side thickness logic in code
- [x] Verified rounded inner cutout logic in code
- [ ] Tested zero-thickness and extreme-radius edge cases visually
- [ ] Documented current geometry constraints

### Base Rendering and Styling

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Solid fill | Implemented | [x] | [x] | [x] | [x] | Base bezel fill is a solid color. | None. |
| Opacity | Implemented | [x] | [x] | [x] | [ ] | Global opacity multiplies rendered colors. | Add docs for interaction with hex alpha. |
| Gradients | Not implemented | [ ] | [ ] | [ ] | [ ] | No gradient code found. | Defer until core polish is complete. |
| Inner shadow | Implemented | [x] | [x] | [x] | [ ] | Separate inner-shadow pass supports falloff and softness. | Add examples and visual tests. |
| Texture support | Not implemented | [ ] | [ ] | [ ] | [ ] | No non-SVG texture or bitmap layer support. | Keep as post-1.0 enhancement. |
| Pattern layer support | Implemented | [x] | [x] | [x] | [ ] | SVG motif stamping is integrated. | Improve docs and runtime validation. |
| Rendering quality | Partially implemented | [x] | [ ] | [ ] | [ ] | Anti-aliasing and bicubic resampling are used; no profiling or image-quality baseline exists. | Add screenshot regression or benchmark process. |
| Clipping correctness | Partially implemented, Needs verification | [x] | [x] | [x] | [ ] | Pattern clip mask exists; broader corner/edge correctness is not validated across many shapes. | Add targeted tests and visual fixtures. |
| Anti-aliasing quality | Partially implemented, Needs verification | [x] | [ ] | [ ] | [ ] | Anti-aliasing is enabled in major passes, but no tuning or validation exists. | Evaluate on fractional scaling and thin geometry. |

Checklist:

- [x] Verified fill, highlight, bezel shadow, and inner shadow code paths
- [x] Verified pattern rendering path with tests
- [ ] Benchmarked redraw cost
- [ ] Documented rendering limitations

### SVG Pattern System

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Asset loading | Implemented | [x] | [x] | [x] | [ ] | Reads SVG file, parses with `usvg`, renders via `resvg`. | Add better user-facing errors. |
| Default SVG directory handling | Implemented | [x] | [x] | [x] | [ ] | Resolves under `XDG_CONFIG_HOME/rustbar/svg` and `~/.config/rustbar/svg`. | Add examples and docs for include-based config layouts. |
| Relative path resolution from `~/.config/rustbar/svg` | Implemented | [x] | [x] | [x] | [ ] | Explicit resolver covers XDG and home config directories. | None. |
| Absolute path handling | Implemented | [x] | [ ] | [x] | [ ] | Absolute paths are accepted directly. | Add tests. |
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
| Failure handling for missing/invalid SVG files | Partially implemented, Needs documentation | [x] | [ ] | [ ] | [ ] | Warns and skips rendering; no surfaced UI/error summary exists. | Document failure modes and add tests. |
| SVG caching | Not implemented | [ ] | [ ] | [ ] | [ ] | SVG is read and parsed during each redraw/reload path. | Add cache keyed by resolved path plus mtime. |

Checklist:

- [x] Verified SVG path resolution in code and tests
- [x] Verified pattern render path in tests
- [ ] Tested all placement modes visually
- [ ] Documented all mode semantics with examples
- [ ] Added caching plan and invalid-asset UX notes

### Configuration and User Experience

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| TOML configuration support | Implemented | [x] | [x] | [x] | [x] | Core config loading is present. | None. |
| Include support | Implemented | [x] | [x] | [ ] | [ ] | `include` and variable expansion work, but docs do not describe them. | Document include/variables schema. |
| Live reload / config reload | Implemented | [x] | [ ] | [x] | [ ] | File watcher reloads all loaded config files. | Add manual and automated reload verification. |
| Validation of invalid config | Partially implemented | [x] | [ ] | [ ] | [ ] | Parse errors skip reload; invalid enum-like strings silently fall back in some cases. | Add schema validation and explicit diagnostics. |
| Fallback behavior | Implemented, Needs documentation | [x] | [x] | [ ] | [ ] | Config path and many values have defaults/fallbacks. | Document fallback order and invalid-value handling. |
| Default values | Implemented | [x] | [x] | [x] | [ ] | Defaults are declared in config structs. | Sync docs tightly to code. |
| Debug logging | Partially implemented | [x] | [ ] | [ ] | [ ] | `env_logger` is initialized; warnings exist, but no documented log strategy. | Document `RUST_LOG` usage and log categories. |
| Readable errors | Partially implemented | [x] | [ ] | [ ] | [ ] | Warnings/errors exist but are inconsistent and mostly log-only. | Improve startup/reload error summaries. |
| Documentation of configuration schema | Partially implemented | [x] | [ ] | [ ] | [ ] | Existing docs are incomplete and slightly ahead of or misaligned with implementation details. | Update docs after roadmap is merged. |

Checklist:

- [x] Verified path resolution order in code and tests
- [x] Verified include and variable expansion in tests
- [ ] Verified live reload end-to-end
- [ ] Documented unsupported or fallback enum values
- [ ] Added examples for per-monitor layer and pattern overrides

### Performance and Runtime Behavior

| Feature | Status | Code | Tested | Docs | Ready | Notes | Next action |
| :--- | :--- | :---: | :---: | :---: | :---: | :--- | :--- |
| Idle CPU behavior | Needs verification | [ ] | [ ] | [ ] | [ ] | Event loop is passive, but Hyprland listener reconnect loop and file watcher behavior are not measured. | Measure idle CPU on target compositors. |
| Redraw behavior | Partially implemented | [x] | [ ] | [ ] | [ ] | Redraw happens on configure and config reload; no frame callback loop is used. | Document redraw triggers and avoid unnecessary rerenders. |
| Caching strategy | Partially implemented | [x] | [ ] | [ ] | [ ] | SHM slot pools are reused per surface, but no higher-level asset/render caches exist. | Add SVG cache and maybe per-config render cache. |
| Geometry recomputation rules | Partially implemented | [x] | [ ] | [ ] | [ ] | Geometry is recomputed on draw/configure, but scale/transform changes are ignored. | Define explicit recomputation triggers. |
| Multi-monitor performance | Needs verification | [x] | [ ] | [ ] | [ ] | One pixmap per surface redraw; no benchmark data. | Measure redraw and memory cost across multiple outputs. |
| Startup behavior | Partially implemented | [x] | [ ] | [ ] | [ ] | Startup path is straightforward but relies on `expect`/`unwrap` for critical failures. | Improve startup diagnostics and document prerequisites. |
| Runtime stability | Partially implemented | [x] | [x] | [ ] | [ ] | Unit tests pass, but runtime integration coverage is minimal. | Add compositor smoke tests and manual QA checklist. |

Checklist:

- [x] Verified slot-pool reuse in code
- [ ] Measured idle CPU
- [ ] Measured redraw cost with patterns enabled
- [ ] Documented performance expectations

### Future-Facing Features

| Feature | Status | Notes | Next action |
| :--- | :--- | :--- | :--- |
| Presets | Partially implemented, Needs refactor | `preset` key exists in config schema but no preset application logic is implemented. | Either remove from public docs for now or implement preset resolution. |
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

- README claims fullscreen handling "hides the frame"; current code only changes the layer on Hyprland fullscreen outputs.
- README does not explain include support, variable expansion, or `loaded_files`-based reload behavior.
- `docs/configuration.md` documents many options but does not distinguish implemented behavior from planned behavior.
- No document explains per-output lifecycle, hotplug behavior, or monitor disable semantics on reload.
- No document explains pattern mode semantics with examples or visual expectations.

### Implemented but under-documented

- Config path resolution order
- Include support and variables
- Namespace behavior
- Click-through implementation details and compositor caveats
- Pattern seed and deterministic jitter
- Blend modes
- `prevent_overlap` semantics
- Pattern clipping behavior
- Absolute SVG path handling

### Likely misleading or incomplete docs

- Fullscreen behavior wording
- Production-readiness implied by feature lists
- Lack of mention that scale and transform handlers are currently empty
- Lack of mention that `preset` exists in schema but is not applied

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
- Test hotplug and multi-monitor behavior manually
- Harden error handling around config reload and asset failures

Dependencies / blockers:

- Requires compositor-level manual testing

Implementation notes:

- Decide whether fullscreen behavior should hide, demote, or fully disable the surface
- Add an active-surface reconciliation step inside output sync

### v1.0: Correctness and Multi-Output Readiness

Goal: establish a trustworthy 1.0 core.

Priority features:

- Implement output scale-factor handling
- Implement output transform/rotation handling
- Add integration-level validation for multi-monitor sessions
- Add SVG caching to avoid repeated parse/render work
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

- Implement real preset application logic or remove `preset` from schema until ready
- Add shipped presets such as CRT-inspired, minimal, and ornamental styles
- Consider texture/pattern composition layers
- Consider dedicated corner motifs

Dependencies / blockers:

- Requires a clear style/preset data model

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

- What is the intended fullscreen product behavior: hide entirely, switch layer, disable input only, or user-selectable policy?
- Should base-frame side enablement remain thickness-driven, or should explicit side booleans be added?
- Is `preset` meant to be a built-in named style system, an importable theme file, or just a future alias layer?
- Should non-Hyprland fullscreen handling exist, and if so, how should compositor support be detected?
- Should SVG patterns be cached as parsed trees, rasterized motifs, or both?
- How should scale-factor and transform changes alter geometry: logical-pixel preservation, physical-pixel preservation, or configurable behavior?

## 8. Recommended Immediate Next Steps

1. Update `README.md` and `docs/configuration.md` so fullscreen, pattern, include, and preset documentation matches the code.
2. Fix reload reconciliation so monitors disabled in config are removed cleanly.
3. Decide and implement the intended fullscreen policy, then test it on Hyprland.
4. Add manual verification notes for click-through, exclusive zone, hotplug, and multi-monitor behavior.
5. Implement scale-factor and transform handling before expanding the feature surface further.
6. Add SVG caching and better asset/config diagnostics once correctness work is in place.

## 9. Contributor Tracking Checklist

### Immediate engineering tasks

- [ ] README corrected to match actual fullscreen behavior
- [ ] `docs/configuration.md` updated with include/variables support
- [ ] `preset` marked as unimplemented or implemented properly
- [ ] Reload removes disabled monitor surfaces
- [ ] Fullscreen behavior changed from current layer-downgrade workaround to intended design
- [ ] Hotplug add/remove tested
- [ ] Click-through tested on at least Hyprland
- [ ] Exclusive-zone behavior tested
- [ ] Scale-factor handler implemented
- [ ] Transform/rotation handler implemented
- [ ] SVG caching implemented
- [ ] Invalid-config diagnostics improved

### Validation tasks

- [x] Unit tests currently pass with `cargo test`
- [ ] Manual QA checklist added to repository
- [ ] Visual regression strategy defined
- [ ] Multi-compositor support matrix documented

### Documentation tasks

- [ ] Feature matrix added to README or linked from README
- [ ] Configuration guide aligned with current schema and behavior
- [ ] Pattern mode examples added
- [ ] Multi-monitor behavior documented
- [ ] Runtime logging and troubleshooting documented
