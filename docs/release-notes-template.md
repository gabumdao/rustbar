# Rustbar Release Notes Template

Use this template when cutting a tagged release. Keep it short, accurate, and limited to behavior that is actually verified in the repository.

## Rustbar vX.Y.Z

Date: YYYY-MM-DD  
Commit: `<commit-hash>`

### Highlights

- Short summary of the biggest user-visible improvement.
- Short summary of the second most important change.
- Short summary of any install, runtime-control, or config workflow improvement.

### Added

- New feature or workflow.
- New command, config capability, or documentation artifact.

### Changed

- Behavior changes that existing users should notice.
- Config or runtime semantics that changed but are not strictly bugfixes.

### Fixed

- Bugfix with user-visible impact.
- Runtime, reload, rendering, or compositor-specific fix.

### Verification Summary

- Automated checks run:
  - `cargo test`
  - `cargo test --bin rustbarctl`
  - `cargo build --release`
- Manual compositor verification completed:
  - Hyprland: yes/no
  - Sway: yes/no
- Performance capture reviewed: yes/no

### Compositor Support Notes

- Hyprland: note the verified behaviors for this release.
- Sway: note whether support is verified, pending, or explicitly unclaimed.
- Other wlroots compositors: note the current support boundary.

### Install and Upgrade Notes

- Any change to `make install`, completion installation, runtime socket usage, or CLI usage.
- Any config migration note or changed default worth calling out.

### Known Limits

- Any compositor-specific limitation still present.
- Any deferred roadmap item users might reasonably assume is ready but is not.
- Any performance or rendering limit still worth mentioning.

### Links

- Roadmap: [`docs/rustbar-roadmap.md`](/home/walkerofdoom/homolog/rustbar/docs/rustbar-roadmap.md)
- Release checklist: [`docs/release-checklist.md`](/home/walkerofdoom/homolog/rustbar/docs/release-checklist.md)
- Verification checklist: [`docs/checklist_roadmap.md`](/home/walkerofdoom/homolog/rustbar/docs/checklist_roadmap.md)
