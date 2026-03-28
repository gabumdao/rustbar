# Rustbar Release Checklist

This checklist is the canonical release handoff for `rustbar`. Use it before cutting a tagged release or publishing install instructions that imply a stable build.

## 1. Scope and Version

- [ ] Decide the release version and confirm the milestone or roadmap scope is actually complete.
- [ ] Review [`docs/rustbar-roadmap.md`](/home/walkerofdoom/homolog/rustbar/docs/rustbar-roadmap.md) and move any unfinished items out of the release scope.
- [ ] Confirm which compositor claims are verified for this release.
- [ ] Write a short release summary covering user-visible features, bugfixes, and known limits.

## 2. Repository Truthfulness

- [ ] Ensure [`README.md`](/home/walkerofdoom/homolog/rustbar/README.md) matches the current feature set.
- [ ] Ensure [`docs/configuration.md`](/home/walkerofdoom/homolog/rustbar/docs/configuration.md) matches the current config schema and runtime behavior.
- [ ] Ensure [`docs/checklist_roadmap.md`](/home/walkerofdoom/homolog/rustbar/docs/checklist_roadmap.md) reflects the latest compositor and runtime verification results.
- [ ] Ensure [`docs/rustbar-roadmap.md`](/home/walkerofdoom/homolog/rustbar/docs/rustbar-roadmap.md) reflects the actual implementation status, not planned status.
- [ ] Confirm any deferred items are clearly marked as deferred, compositor-specific, partial, or future work.

## 3. Automated Verification

Run these from the repository root:

```bash
cargo test
cargo test --bin rustbarctl
cargo build --release
```

Checklist:

- [ ] `cargo test` passes.
- [ ] `cargo test --bin rustbarctl` passes.
- [ ] `cargo build --release` succeeds.
- [ ] `make -n install` still produces the expected install plan.
- [ ] `sh -n scripts/rustbarctl.sh` passes.
- [ ] `sh -n scripts/install-completions.sh` passes.
- [ ] Bash completion syntax check passes: `bash -n completions/rustbarctl.bash`.
- [ ] Zsh completion syntax check passes when `zsh` is available: `zsh -n completions/_rustbarctl`.
- [ ] Fish completion syntax check passes when `fish` is available: `fish --no-execute completions/rustbarctl.fish`.

## 4. Runtime Verification

### Hyprland

Use [`docs/checklist_roadmap.md`](/home/walkerofdoom/homolog/rustbar/docs/checklist_roadmap.md) as the source of truth.

- [ ] Baseline startup is still clean.
- [ ] Layer selection coverage is still valid.
- [ ] Click-through, focus avoidance, exclusive zone, and fullscreen behavior still pass.
- [ ] Hot reload still works.
- [ ] Runtime IPC commands still behave correctly.
- [ ] Output-scoped visibility commands still behave correctly.
- [ ] Any new feature added since the last release has manual verification notes.

### Other compositors

- [ ] Sway verification is run if this release will claim Sway support.
- [ ] Any compositor-specific limitations are written down explicitly.

## 5. Performance and Stability

- [ ] Run the current performance pass if rendering, caching, or runtime control changed.
- [ ] Capture at least one release-build run with `scripts/perf_capture.sh --release` when performance-sensitive code changed.
- [ ] Review `output.log` for redraw timing, CPU behavior, and runtime warnings.
- [ ] Confirm no known runtime warning regression reappeared.
- [ ] Confirm simple config edits still reload quickly enough in release builds.

## 6. Packaging and Install Path

- [ ] `make install` installs both `rustbar` and `rustbarctl` into the expected prefix.
- [ ] `scripts/install-completions.sh --all` still installs shell completions into the expected locations.
- [ ] Installation instructions still match the current binaries, helper scripts, and completion files.
- [ ] Any new runtime assets or helper scripts are included in the documented install path.

## 7. Release Notes and Known Limits

Before tagging a release, write down:

- [ ] User-visible features added since the last release.
- [ ] Important bugfixes.
- [ ] Known compositor-specific limits.
- [ ] Known performance limitations, if any remain.
- [ ] Any deferred roadmap items that users might expect but should not assume are ready.

## 8. Tagging and Publish Handoff

- [ ] Create or update the changelog or release notes entry.
- [ ] Start from [`docs/release-notes-template.md`](/home/walkerofdoom/homolog/rustbar/docs/release-notes-template.md) so release notes stay consistent.
- [ ] Tag the release only after verification items above are complete.
- [ ] Record the exact commit hash associated with the release.
- [ ] If binaries or packages are published elsewhere, confirm the published artifacts were built from the tagged commit.

## 9. Minimum Ship Bar

Do not cut a release if any of these are false:

- [ ] Tests pass.
- [ ] Release build succeeds.
- [ ] README and configuration docs are accurate.
- [ ] Hyprland verification reflects the current code.
- [ ] Unsupported or unverified compositor behavior is clearly marked.
- [ ] Runtime control, install path, and completion docs match the current binaries.
