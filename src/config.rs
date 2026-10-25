use log::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default = "default_thickness")]
    pub thickness: u32,
    pub thickness_top: Option<u32>,
    pub thickness_bottom: Option<u32>,
    pub thickness_left: Option<u32>,
    pub thickness_right: Option<u32>,
    #[serde(default)]
    pub frame_sides: FrameSidesConfig,
    #[serde(default = "default_margin")]
    pub margin: u32,
    pub margin_top: Option<u32>,
    pub margin_bottom: Option<u32>,
    pub margin_left: Option<u32>,
    pub margin_right: Option<u32>,
    #[serde(default = "default_offset")]
    pub offset_x: i32,
    #[serde(default = "default_offset")]
    pub offset_y: i32,
    #[serde(default = "default_radius")]
    pub radius: u32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub preset: Option<String>,

    #[serde(default = "default_shadow_width")]
    pub shadow_width: f32,
    #[serde(default = "default_shadow_color")]
    pub shadow_color: String,

    #[serde(default = "default_highlight_width")]
    pub highlight_width: f32,
    #[serde(default = "default_highlight_color")]
    pub highlight_color: String,

    #[serde(default)]
    pub inner_shadow: InnerShadowConfig,

    #[serde(default)]
    pub gradient: GradientConfig,

    #[serde(default)]
    pub layer: LayerConfig,

    #[serde(default)]
    pub pattern: PatternConfig,

    #[serde(default)]
    pub monitors: HashMap<String, MonitorConfig>,

    #[serde(skip)]
    pub loaded_files: Vec<std::path::PathBuf>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternConfig {
    #[serde(default = "default_pattern_enabled")]
    pub enabled: bool,
    pub source: Option<String>,
    #[serde(default = "default_pattern_mode")]
    pub mode: String,
    #[serde(default = "default_pattern_opacity")]
    pub opacity: f32,
    pub color: Option<String>,
    #[serde(default = "default_pattern_blend")]
    pub blend: String,
    #[serde(default = "default_pattern_seed")]
    pub seed: u64,

    #[serde(default)]
    pub transform: PatternTransformConfig,
    #[serde(default)]
    pub repeat: PatternRepeatConfig,
    #[serde(default)]
    pub sides: PatternSidesConfig,
    #[serde(default)]
    pub sources: PatternSourcesConfig,
    #[serde(default)]
    pub corners: PatternCornersConfig,
    #[serde(default)]
    pub layout: PatternLayoutConfig,
}

fn default_pattern_enabled() -> bool {
    false
}
fn default_pattern_mode() -> String {
    "repeat".to_string()
}
fn default_pattern_opacity() -> f32 {
    1.0
}
fn default_pattern_blend() -> String {
    "normal".to_string()
}
fn default_pattern_seed() -> u64 {
    0
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            enabled: default_pattern_enabled(),
            source: None,
            mode: default_pattern_mode(),
            opacity: default_pattern_opacity(),
            color: None,
            blend: default_pattern_blend(),
            seed: default_pattern_seed(),
            transform: PatternTransformConfig::default(),
            repeat: PatternRepeatConfig::default(),
            sides: PatternSidesConfig::default(),
            sources: PatternSourcesConfig::default(),
            corners: PatternCornersConfig::default(),
            layout: PatternLayoutConfig::default(),
        }
    }
}

impl PatternConfig {
    pub fn resolve_svg_path(source: &str) -> std::path::PathBuf {
        if let Some(stripped) = source.strip_prefix("~/") {
            if let Ok(home) = std::env::var("HOME") {
                let expanded = Path::new(&home).join(stripped);
                if expanded.exists() {
                    return expanded;
                }
            }
        }

        let path = Path::new(source);
        if path.is_absolute() {
            return path.to_path_buf();
        }

        let mut search_dirs = Vec::new();
        if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            search_dirs.push(Path::new(&xdg_config_home).join("rustbar").join("svg"));
        }
        if let Ok(home) = std::env::var("HOME") {
            search_dirs.push(Path::new(&home).join(".config").join("rustbar").join("svg"));
        }
        search_dirs.push(Path::new(".").to_path_buf());

        for dir in search_dirs {
            let candidate = dir.join(source);
            if candidate.exists() {
                return candidate;
            }
            if !source.ends_with(".svg") {
                let candidate_svg = dir.join(format!("{}.svg", source));
                if candidate_svg.exists() {
                    return candidate_svg;
                }
            }
        }

        let path_buf = path.to_path_buf();
        if !path_buf.exists() && !source.ends_with(".svg") {
            let fallback_buf = Path::new(&format!("{}.svg", source)).to_path_buf();
            if fallback_buf.exists() {
                return fallback_buf;
            }
        }

        path_buf
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternTransformConfig {
    #[serde(default = "default_pattern_scale")]
    pub scale: f32,
    #[serde(default = "default_pattern_rotation")]
    pub rotation: f32,
    #[serde(default = "default_pattern_flip_x")]
    pub flip_x: bool,
    #[serde(default = "default_pattern_flip_y")]
    pub flip_y: bool,
}

fn default_pattern_scale() -> f32 {
    1.0
}
fn default_pattern_rotation() -> f32 {
    0.0
}
fn default_pattern_flip_x() -> bool {
    false
}
fn default_pattern_flip_y() -> bool {
    false
}

impl Default for PatternTransformConfig {
    fn default() -> Self {
        Self {
            scale: default_pattern_scale(),
            rotation: default_pattern_rotation(),
            flip_x: default_pattern_flip_x(),
            flip_y: default_pattern_flip_y(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternRepeatConfig {
    #[serde(default = "default_pattern_spacing")]
    pub spacing: f32,
    #[serde(default = "default_pattern_offset")]
    pub offset: f32,
    #[serde(default = "default_pattern_jitter_spacing")]
    pub jitter_spacing: f32,
    #[serde(default = "default_pattern_jitter_rotation")]
    pub jitter_rotation: f32,
    #[serde(default = "default_pattern_jitter_scale")]
    pub jitter_scale: f32,
}

fn default_pattern_spacing() -> f32 {
    24.0
}
fn default_pattern_offset() -> f32 {
    0.0
}
fn default_pattern_jitter_spacing() -> f32 {
    0.0
}
fn default_pattern_jitter_rotation() -> f32 {
    0.0
}
fn default_pattern_jitter_scale() -> f32 {
    0.0
}

impl Default for PatternRepeatConfig {
    fn default() -> Self {
        Self {
            spacing: default_pattern_spacing(),
            offset: default_pattern_offset(),
            jitter_spacing: default_pattern_jitter_spacing(),
            jitter_rotation: default_pattern_jitter_rotation(),
            jitter_scale: default_pattern_jitter_scale(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct FrameSidesConfig {
    #[serde(default = "default_frame_side_enabled")]
    pub top: bool,
    #[serde(default = "default_frame_side_enabled")]
    pub bottom: bool,
    #[serde(default = "default_frame_side_enabled")]
    pub left: bool,
    #[serde(default = "default_frame_side_enabled")]
    pub right: bool,
}

fn default_frame_side_enabled() -> bool {
    true
}

impl Default for FrameSidesConfig {
    fn default() -> Self {
        Self {
            top: default_frame_side_enabled(),
            bottom: default_frame_side_enabled(),
            left: default_frame_side_enabled(),
            right: default_frame_side_enabled(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorFrameSidesConfig {
    pub top: Option<bool>,
    pub bottom: Option<bool>,
    pub left: Option<bool>,
    pub right: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternSidesConfig {
    #[serde(default = "default_pattern_side_enabled")]
    pub top: bool,
    #[serde(default = "default_pattern_side_enabled")]
    pub bottom: bool,
    #[serde(default = "default_pattern_side_enabled")]
    pub left: bool,
    #[serde(default = "default_pattern_side_enabled")]
    pub right: bool,
}

fn default_pattern_side_enabled() -> bool {
    true
}

impl Default for PatternSidesConfig {
    fn default() -> Self {
        Self {
            top: default_pattern_side_enabled(),
            bottom: default_pattern_side_enabled(),
            left: default_pattern_side_enabled(),
            right: default_pattern_side_enabled(),
        }
    }
}


#[derive(Debug, Deserialize, Clone, PartialEq, Default)]
pub struct PatternSourcesConfig {
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Default)]
pub struct MonitorPatternSourcesConfig {
    pub top: Option<String>,
    pub bottom: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternCornersConfig {
    #[serde(default = "default_pattern_avoid_corners")]
    pub avoid_corners: bool,
    #[serde(default = "default_pattern_corner_padding")]
    pub corner_padding: f32,
    pub top_left: Option<String>,
    pub top_right: Option<String>,
    pub bottom_left: Option<String>,
    pub bottom_right: Option<String>,
}

fn default_pattern_avoid_corners() -> bool {
    false
}
fn default_pattern_corner_padding() -> f32 {
    12.0
}

impl Default for PatternCornersConfig {
    fn default() -> Self {
        Self {
            avoid_corners: default_pattern_avoid_corners(),
            corner_padding: default_pattern_corner_padding(),
            top_left: None,
            top_right: None,
            bottom_left: None,
            bottom_right: None,
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternLayoutConfig {
    #[serde(default = "default_pattern_follow_edge")]
    pub follow_edge_direction: bool,
    #[serde(default = "default_pattern_clip")]
    pub clip_to_frame: bool,
    #[serde(default = "default_pattern_prevent_overlap")]
    pub prevent_overlap: bool,
    #[serde(default = "default_pattern_overlap_padding")]
    pub overlap_padding: f32,
    #[serde(default = "default_pattern_angle")]
    pub angle: f32,
}

fn default_pattern_follow_edge() -> bool {
    true
}
fn default_pattern_clip() -> bool {
    true
}
fn default_pattern_prevent_overlap() -> bool {
    false
}
fn default_pattern_overlap_padding() -> f32 {
    2.0
}
fn default_pattern_angle() -> f32 {
    45.0
}

impl Default for PatternLayoutConfig {
    fn default() -> Self {
        Self {
            follow_edge_direction: default_pattern_follow_edge(),
            clip_to_frame: default_pattern_clip(),
            prevent_overlap: default_pattern_prevent_overlap(),
            overlap_padding: default_pattern_overlap_padding(),
            angle: default_pattern_angle(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct GradientConfig {
    #[serde(default = "default_gradient_enabled")]
    pub enabled: bool,
    #[serde(default = "default_gradient_mode")]
    pub mode: String,
    #[serde(default = "default_gradient_start_color")]
    pub start_color: String,
    #[serde(default = "default_gradient_end_color")]
    pub end_color: String,
}

fn default_gradient_enabled() -> bool {
    false
}
fn default_gradient_mode() -> String {
    "vertical".to_string()
}
fn default_gradient_start_color() -> String {
    "#000000".to_string()
}
fn default_gradient_end_color() -> String {
    "#202020".to_string()
}

impl Default for GradientConfig {
    fn default() -> Self {
        Self {
            enabled: default_gradient_enabled(),
            mode: default_gradient_mode(),
            start_color: default_gradient_start_color(),
            end_color: default_gradient_end_color(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorGradientConfig {
    pub enabled: Option<bool>,
    pub mode: Option<String>,
    pub start_color: Option<String>,
    pub end_color: Option<String>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct InnerShadowConfig {
    #[serde(default = "default_inner_shadow_enabled")]
    pub enabled: bool,
    #[serde(default = "default_inner_shadow_size")]
    pub size: f32,
    #[serde(default = "default_inner_shadow_opacity")]
    pub opacity: f32,
    #[serde(default = "default_inner_shadow_color")]
    pub color: String,
    #[serde(default = "default_inner_shadow_softness")]
    pub softness: f32,
    #[serde(default = "default_inner_shadow_falloff")]
    pub falloff: String,
}

fn default_inner_shadow_enabled() -> bool {
    false
}
fn default_inner_shadow_size() -> f32 {
    10.0
}
fn default_inner_shadow_opacity() -> f32 {
    0.5
}
fn default_inner_shadow_color() -> String {
    "#000000".to_string()
}
fn default_inner_shadow_softness() -> f32 {
    1.0
}
fn default_inner_shadow_falloff() -> String {
    "ease_out".to_string()
}

impl Default for InnerShadowConfig {
    fn default() -> Self {
        Self {
            enabled: default_inner_shadow_enabled(),
            size: default_inner_shadow_size(),
            opacity: default_inner_shadow_opacity(),
            color: default_inner_shadow_color(),
            softness: default_inner_shadow_softness(),
            falloff: default_inner_shadow_falloff(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct LayerConfig {
    #[serde(default = "default_layer_level")]
    pub level: String,
    #[serde(default = "default_exclusive_zone")]
    pub exclusive_zone: i32,
    #[serde(default = "default_keyboard_interactivity")]
    pub keyboard_interactivity: String,
    #[serde(default = "default_click_through")]
    pub click_through: bool,
    #[serde(default = "default_namespace")]
    pub namespace: String,
}

fn default_layer_level() -> String {
    "bottom".to_string()
}
fn default_exclusive_zone() -> i32 {
    0
}
fn default_keyboard_interactivity() -> String {
    "none".to_string()
}
fn default_click_through() -> bool {
    true
}
fn default_namespace() -> String {
    "rustbar-frame".to_string()
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            level: default_layer_level(),
            exclusive_zone: default_exclusive_zone(),
            keyboard_interactivity: default_keyboard_interactivity(),
            click_through: default_click_through(),
            namespace: default_namespace(),
        }
    }
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorConfig {
    pub enabled: Option<bool>,
    pub thickness: Option<u32>,
    pub thickness_top: Option<u32>,
    pub thickness_bottom: Option<u32>,
    pub thickness_left: Option<u32>,
    pub thickness_right: Option<u32>,
    pub frame_sides: Option<MonitorFrameSidesConfig>,
    pub margin: Option<u32>,
    pub margin_top: Option<u32>,
    pub margin_bottom: Option<u32>,
    pub margin_left: Option<u32>,
    pub margin_right: Option<u32>,
    pub offset_x: Option<i32>,
    pub offset_y: Option<i32>,
    pub radius: Option<u32>,
    pub opacity: Option<f32>,
    pub color: Option<String>,
    pub preset: Option<String>,
    pub shadow_width: Option<f32>,
    pub shadow_color: Option<String>,
    pub highlight_width: Option<f32>,
    pub highlight_color: Option<String>,
    pub inner_shadow: Option<MonitorInnerShadowConfig>,
    pub gradient: Option<MonitorGradientConfig>,
    pub pattern: Option<MonitorPatternConfig>,
    pub layer: Option<LayerConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorPatternConfig {
    pub enabled: Option<bool>,
    pub source: Option<String>,
    pub sources: Option<MonitorPatternSourcesConfig>,
    pub mode: Option<String>,
    pub opacity: Option<f32>,
    pub color: Option<String>,
    pub blend: Option<String>,
    pub seed: Option<u64>,
    pub transform: Option<PatternTransformConfig>,
    pub repeat: Option<PatternRepeatConfig>,
    pub sides: Option<PatternSidesConfig>,
    pub corners: Option<PatternCornersConfig>,
    pub layout: Option<PatternLayoutConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorInnerShadowConfig {
    pub enabled: Option<bool>,
    pub size: Option<f32>,
    pub opacity: Option<f32>,
    pub color: Option<String>,
    pub softness: Option<f32>,
    pub falloff: Option<String>,
}

fn default_thickness() -> u32 {
    16
}
fn default_margin() -> u32 {
    0
}
fn default_offset() -> i32 {
    0
}
fn default_radius() -> u32 {
    24
}
fn default_opacity() -> f32 {
    1.0
}
fn default_color() -> String {
    "#000000".to_string()
}
fn default_shadow_width() -> f32 {
    12.0
}
fn default_shadow_color() -> String {
    "#000000AA".to_string()
}
fn default_highlight_width() -> f32 {
    3.0
}
fn default_highlight_color() -> String {
    "#FFFFFF22".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            thickness: default_thickness(),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: FrameSidesConfig::default(),
            margin: default_margin(),
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: default_offset(),
            offset_y: default_offset(),
            radius: default_radius(),
            opacity: default_opacity(),
            color: default_color(),
            preset: None,
            shadow_width: default_shadow_width(),
            shadow_color: default_shadow_color(),
            highlight_width: default_highlight_width(),
            highlight_color: default_highlight_color(),
            inner_shadow: InnerShadowConfig::default(),
            gradient: GradientConfig::default(),
            pattern: PatternConfig::default(),
            layer: LayerConfig::default(),
            monitors: HashMap::new(),
            loaded_files: Vec::new(),
        }
    }
}

fn is_valid_layer_level(level: &str) -> bool {
    matches!(
        level.to_lowercase().as_str(),
        "background" | "bottom" | "top" | "overlay"
    )
}

fn is_valid_keyboard_interactivity(value: &str) -> bool {
    matches!(
        value.to_lowercase().as_str(),
        "none" | "on_demand" | "exclusive"
    )
}

fn is_valid_pattern_mode(mode: &str) -> bool {
    matches!(
        mode.to_lowercase().as_str(),
        "repeat"
            | "single"
            | "scatter"
            | "horizontal"
            | "vertical"
            | "diagonal"
            | "diagonal_repeat"
            | "diagonal_lines"
            | "checker"
            | "diamond"
    )
}

fn is_valid_pattern_blend(blend: &str) -> bool {
    matches!(
        blend.to_lowercase().as_str(),
        "normal" | "multiply" | "screen" | "overlay" | "darken" | "lighten"
    )
}

fn is_valid_inner_shadow_falloff(falloff: &str) -> bool {
    matches!(
        falloff.to_lowercase().as_str(),
        "ease_out" | "ease_in" | "linear"
    )
}

fn is_valid_gradient_mode(mode: &str) -> bool {
    matches!(
        mode.to_lowercase().as_str(),
        "vertical" | "horizontal" | "diagonal_down" | "diagonal_up"
    )
}

fn collect_pattern_source_entries(pattern: &MonitorPatternConfig) -> Vec<(&'static str, &str)> {
    let mut entries = Vec::new();
    if let Some(source) = pattern.source.as_deref() {
        entries.push(("pattern.source", source));
    }
    if let Some(sources) = &pattern.sources {
        if let Some(source) = sources.top.as_deref() {
            entries.push(("pattern.sources.top", source));
        }
        if let Some(source) = sources.bottom.as_deref() {
            entries.push(("pattern.sources.bottom", source));
        }
        if let Some(source) = sources.left.as_deref() {
            entries.push(("pattern.sources.left", source));
        }
        if let Some(source) = sources.right.as_deref() {
            entries.push(("pattern.sources.right", source));
        }
    }
    if let Some(corners) = &pattern.corners {
        if let Some(source) = corners.top_left.as_deref() {
            entries.push(("pattern.corners.top_left", source));
        }
        if let Some(source) = corners.top_right.as_deref() {
            entries.push(("pattern.corners.top_right", source));
        }
        if let Some(source) = corners.bottom_left.as_deref() {
            entries.push(("pattern.corners.bottom_left", source));
        }
        if let Some(source) = corners.bottom_right.as_deref() {
            entries.push(("pattern.corners.bottom_right", source));
        }
    }
    entries
}

fn table_without_keys(
    table: &toml::map::Map<String, toml::Value>,
    excluded_keys: &[&str],
) -> toml::map::Map<String, toml::Value> {
    table
        .iter()
        .filter(|(key, _)| !excluded_keys.iter().any(|excluded| key == excluded))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn apply_named_preset(
    target: &mut toml::map::Map<String, toml::Value>,
    presets: &toml::map::Map<String, toml::Value>,
    context: &str,
    excluded_keys: &[&str],
    warnings: &mut Vec<String>,
) {
    let Some(preset_name) = target
        .get("preset")
        .and_then(|value| value.as_str())
        .map(str::to_string)
    else {
        return;
    };

    let Some(preset_value) = presets.get(&preset_name) else {
        warnings.push(format!(
            "{} sets preset='{}', but no matching preset definition exists",
            context, preset_name
        ));
        return;
    };

    let Some(preset_table) = preset_value.as_table() else {
        warnings.push(format!(
            "preset '{}' for {} is not a TOML table and will be ignored",
            preset_name, context
        ));
        return;
    };

    let mut merged = toml::Value::Table(table_without_keys(preset_table, &["preset", "presets", "monitors", "include"]));
    merge_toml_values(
        &mut merged,
        toml::Value::Table(table_without_keys(target, excluded_keys)),
    );

    if let toml::Value::Table(merged_table) = merged {
        for (key, value) in merged_table {
            target.insert(key, value);
        }
    }
}

fn apply_presets_to_config_value(value: &mut toml::Value) -> Vec<String> {
    let mut warnings = Vec::new();

    let Some(root) = value.as_table_mut() else {
        return warnings;
    };

    let Some(presets_value) = root.get("presets") else {
        return warnings;
    };

    let Some(presets) = presets_value.as_table().cloned() else {
        warnings.push("top-level presets must be a TOML table; preset application will be skipped".to_string());
        return warnings;
    };

    apply_named_preset(
        root,
        &presets,
        "global config",
        &["monitors", "presets", "include"],
        &mut warnings,
    );

    if let Some(monitors) = root.get_mut("monitors").and_then(|value| value.as_table_mut()) {
        for (name, monitor_value) in monitors.iter_mut() {
            match monitor_value.as_table_mut() {
                Some(monitor_table) => apply_named_preset(
                    monitor_table,
                    &presets,
                    &format!("monitor '{}'", name),
                    &[],
                    &mut warnings,
                ),
                None => warnings.push(format!(
                    "monitor '{}' config is not a TOML table; preset application will be skipped",
                    name
                )),
            }
        }
    }

    warnings
}

fn collect_monitor_validation_warnings(name: &str, config: &MonitorConfig) -> Vec<String> {
    let mut warnings = Vec::new();
    let prefix = if name == "<global>" {
        "global config".to_string()
    } else {
        format!("monitor '{}'", name)
    };

    if let Some(opacity) = config.opacity {
        if !(0.0..=1.0).contains(&opacity) {
            warnings.push(format!(
                "{} uses opacity={}, expected a value between 0.0 and 1.0",
                prefix, opacity
            ));
        }
    }

    if let Some(layer) = &config.layer {
        if !is_valid_layer_level(&layer.level) {
            warnings.push(format!(
                "{} uses unsupported layer.level='{}'; rustbar will fall back to 'bottom'",
                prefix, layer.level
            ));
        }
        if !is_valid_keyboard_interactivity(&layer.keyboard_interactivity) {
            warnings.push(format!(
                "{} uses unsupported layer.keyboard_interactivity='{}'; rustbar will fall back to 'none'",
                prefix, layer.keyboard_interactivity
            ));
        }
    }

    if let Some(inner_shadow) = &config.inner_shadow {
        if let Some(opacity) = inner_shadow.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                warnings.push(format!(
                    "{} uses inner_shadow.opacity={}, expected a value between 0.0 and 1.0",
                    prefix, opacity
                ));
            }
        }
        if let Some(softness) = inner_shadow.softness {
            if softness <= 0.0 {
                warnings.push(format!(
                    "{} uses inner_shadow.softness={}, but rustbar clamps it above 0.0",
                    prefix, softness
                ));
            }
        }
        if let Some(falloff) = &inner_shadow.falloff {
            if !is_valid_inner_shadow_falloff(falloff) {
                warnings.push(format!(
                    "{} uses unsupported inner_shadow.falloff='{}'; rustbar will fall back to 'ease_out'",
                    prefix, falloff
                ));
            }
        }
    }

    if let Some(gradient) = &config.gradient {
        if let Some(mode) = &gradient.mode {
            if !is_valid_gradient_mode(mode) {
                warnings.push(format!(
                    "{} uses unsupported gradient.mode='{}'; rustbar will fall back to 'vertical'",
                    prefix, mode
                ));
            }
        }
    }

    if let Some(pattern) = &config.pattern {
        if pattern.enabled.unwrap_or(false) {
            let sources = collect_pattern_source_entries(pattern);
            if sources.is_empty() {
                warnings.push(format!(
                    "{} enables the pattern layer without setting pattern.source or pattern.sources.<side>",
                    prefix
                ));
            } else {
                for (label, source) in sources {
                    let resolved = PatternConfig::resolve_svg_path(source);
                    if !resolved.exists() {
                        warnings.push(format!(
                            "{} enables {}='{}', but the resolved SVG path '{}' does not exist",
                            prefix,
                            label,
                            source,
                            resolved.display()
                        ));
                    }
                }
            }
        }

        if let Some(mode) = &pattern.mode {
            if !is_valid_pattern_mode(mode) {
                warnings.push(format!(
                    "{} uses unsupported pattern.mode='{}'; rustbar may render it like the default mode",
                    prefix, mode
                ));
            }
        }
        if let Some(blend) = &pattern.blend {
            if !is_valid_pattern_blend(blend) {
                warnings.push(format!(
                    "{} uses unsupported pattern.blend='{}'; rustbar will fall back to 'normal'",
                    prefix, blend
                ));
            }
        }
        if let Some(opacity) = pattern.opacity {
            if !(0.0..=1.0).contains(&opacity) {
                warnings.push(format!(
                    "{} uses pattern.opacity={}, expected a value between 0.0 and 1.0",
                    prefix, opacity
                ));
            }
        }
        if let Some(transform) = &pattern.transform {
            if transform.scale <= 0.0 {
                warnings.push(format!(
                    "{} uses pattern.transform.scale={}, but rustbar clamps scale to a small positive value",
                    prefix, transform.scale
                ));
            }
        }
    }

    warnings
}

impl Config {
    pub fn load_default() -> Self {
        match Self::try_load(&Self::resolve_path()) {
            Ok(cfg) => cfg,
            Err(e) => {
                info!("Could not read or parse config: {}", e);
                Config::default()
            }
        }
    }

    pub fn try_load_default() -> Result<Self, String> {
        Self::try_load(&Self::resolve_path())
    }

    pub fn resolve_path() -> std::path::PathBuf {
        if let Ok(explicit) = std::env::var("RUSTBAR_CONFIG") {
            let explicit_path = std::path::PathBuf::from(explicit);
            if explicit_path.exists() {
                return explicit_path;
            }
        }

        if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            let xdg_path = std::path::Path::new(&xdg_config_home)
                .join("rustbar")
                .join("rustbar.toml");
            if xdg_path.exists() {
                return xdg_path;
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let home_path = std::path::Path::new(&home)
                .join(".config")
                .join("rustbar")
                .join("rustbar.toml");
            if home_path.exists() {
                return home_path;
            }
        }

        let local = std::path::Path::new("rustbar.toml");
        if local.exists() {
            return local.to_path_buf();
        }

        std::path::PathBuf::from("rustbar.toml")
    }

    pub fn load(path: &Path) -> Self {
        match Self::try_load(path) {
            Ok(cfg) => cfg,
            Err(e) => {
                info!(
                    "Could not read or parse config at {}: {}",
                    path.display(),
                    e
                );
                Config::default()
            }
        }
    }

    pub fn try_load(path: &Path) -> Result<Self, String> {
        let mut loaded_files = Vec::new();
        let config_value = load_value_with_includes(path, &mut Vec::new(), &mut loaded_files)
            .map_err(|e| format!("{}: {}", path.display(), e))?;

        let mut config_value = config_value;
        for warning in apply_presets_to_config_value(&mut config_value) {
            warn!("Config warning: {}", warning);
        }

        merge_global_monitor_sections(&mut config_value);

        let mut cfg: Config = config_value
            .try_into()
            .map_err(|e| format!("{}: {}", path.display(), e))?;
        cfg.loaded_files = loaded_files;

        for warning in collect_monitor_validation_warnings("<global>", &cfg.for_monitor("<global>"))
        {
            warn!("Config warning: {}", warning);
        }
        for monitor_name in cfg.monitors.keys() {
            for warning in
                collect_monitor_validation_warnings(monitor_name, &cfg.for_monitor(monitor_name))
            {
                warn!("Config warning: {}", warning);
            }
        }

        Ok(cfg)
    }

    pub fn for_monitor(&self, name: &str) -> MonitorConfig {
        let global = MonitorConfig {
            enabled: Some(true),
            thickness: Some(self.thickness),
            thickness_top: self.thickness_top,
            thickness_bottom: self.thickness_bottom,
            thickness_left: self.thickness_left,
            thickness_right: self.thickness_right,
            frame_sides: Some(MonitorFrameSidesConfig {
                top: Some(self.frame_sides.top),
                bottom: Some(self.frame_sides.bottom),
                left: Some(self.frame_sides.left),
                right: Some(self.frame_sides.right),
            }),
            margin: Some(self.margin),
            margin_top: self.margin_top,
            margin_bottom: self.margin_bottom,
            margin_left: self.margin_left,
            margin_right: self.margin_right,
            offset_x: Some(self.offset_x),
            offset_y: Some(self.offset_y),
            radius: Some(self.radius),
            opacity: Some(self.opacity),
            color: Some(self.color.clone()),
            preset: self.preset.clone(),
            shadow_width: Some(self.shadow_width),
            shadow_color: Some(self.shadow_color.clone()),
            highlight_width: Some(self.highlight_width),
            highlight_color: Some(self.highlight_color.clone()),
            inner_shadow: Some(MonitorInnerShadowConfig {
                enabled: Some(self.inner_shadow.enabled),
                size: Some(self.inner_shadow.size),
                opacity: Some(self.inner_shadow.opacity),
                color: Some(self.inner_shadow.color.clone()),
                softness: Some(self.inner_shadow.softness),
                falloff: Some(self.inner_shadow.falloff.clone()),
            }),
            gradient: Some(MonitorGradientConfig {
                enabled: Some(self.gradient.enabled),
                mode: Some(self.gradient.mode.clone()),
                start_color: Some(self.gradient.start_color.clone()),
                end_color: Some(self.gradient.end_color.clone()),
            }),
            pattern: Some(MonitorPatternConfig {
                enabled: Some(self.pattern.enabled),
                source: self.pattern.source.clone(),
                sources: Some(MonitorPatternSourcesConfig {
                    top: self.pattern.sources.top.clone(),
                    bottom: self.pattern.sources.bottom.clone(),
                    left: self.pattern.sources.left.clone(),
                    right: self.pattern.sources.right.clone(),
                }),
                mode: Some(self.pattern.mode.clone()),
                opacity: Some(self.pattern.opacity),
                color: self.pattern.color.clone(),
                blend: Some(self.pattern.blend.clone()),
                seed: Some(self.pattern.seed),
                transform: Some(self.pattern.transform.clone()),
                repeat: Some(self.pattern.repeat.clone()),
                sides: Some(self.pattern.sides.clone()),
                corners: Some(self.pattern.corners.clone()),
                layout: Some(self.pattern.layout.clone()),
            }),
            layer: Some(self.layer.clone()),
        };

        if let Some(spec) = self.monitors.get(name) {
            MonitorConfig {
                enabled: spec.enabled.or(global.enabled),
                thickness: spec.thickness.or(global.thickness),
                thickness_top: spec.thickness_top.or(global.thickness_top),
                thickness_bottom: spec.thickness_bottom.or(global.thickness_bottom),
                thickness_left: spec.thickness_left.or(global.thickness_left),
                thickness_right: spec.thickness_right.or(global.thickness_right),
                frame_sides: Some(MonitorFrameSidesConfig {
                    top: spec
                        .frame_sides
                        .as_ref()
                        .and_then(|s| s.top)
                        .or(Some(self.frame_sides.top)),
                    bottom: spec
                        .frame_sides
                        .as_ref()
                        .and_then(|s| s.bottom)
                        .or(Some(self.frame_sides.bottom)),
                    left: spec
                        .frame_sides
                        .as_ref()
                        .and_then(|s| s.left)
                        .or(Some(self.frame_sides.left)),
                    right: spec
                        .frame_sides
                        .as_ref()
                        .and_then(|s| s.right)
                        .or(Some(self.frame_sides.right)),
                }),
                margin: spec.margin.or(global.margin),
                margin_top: spec.margin_top.or(global.margin_top),
                margin_bottom: spec.margin_bottom.or(global.margin_bottom),
                margin_left: spec.margin_left.or(global.margin_left),
                margin_right: spec.margin_right.or(global.margin_right),
                offset_x: spec.offset_x.or(global.offset_x),
                offset_y: spec.offset_y.or(global.offset_y),
                radius: spec.radius.or(global.radius),
                opacity: spec.opacity.or(global.opacity),
                color: spec.color.clone().or(global.color),
                preset: spec.preset.clone().or(global.preset),
                shadow_width: spec.shadow_width.or(global.shadow_width),
                shadow_color: spec.shadow_color.clone().or(global.shadow_color),
                highlight_width: spec.highlight_width.or(global.highlight_width),
                highlight_color: spec.highlight_color.clone().or(global.highlight_color),
                inner_shadow: Some(MonitorInnerShadowConfig {
                    enabled: spec
                        .inner_shadow
                        .as_ref()
                        .and_then(|s| s.enabled)
                        .or(Some(self.inner_shadow.enabled)),
                    size: spec
                        .inner_shadow
                        .as_ref()
                        .and_then(|s| s.size)
                        .or(Some(self.inner_shadow.size)),
                    opacity: spec
                        .inner_shadow
                        .as_ref()
                        .and_then(|s| s.opacity)
                        .or(Some(self.inner_shadow.opacity)),
                    color: spec
                        .inner_shadow
                        .as_ref()
                        .and_then(|s| s.color.clone())
                        .or(Some(self.inner_shadow.color.clone())),
                    softness: spec
                        .inner_shadow
                        .as_ref()
                        .and_then(|s| s.softness)
                        .or(Some(self.inner_shadow.softness)),
                    falloff: spec
                        .inner_shadow
                        .as_ref()
                        .and_then(|s| s.falloff.clone())
                        .or(Some(self.inner_shadow.falloff.clone())),
                }),
                gradient: Some(MonitorGradientConfig {
                    enabled: spec
                        .gradient
                        .as_ref()
                        .and_then(|s| s.enabled)
                        .or(Some(self.gradient.enabled)),
                    mode: spec
                        .gradient
                        .as_ref()
                        .and_then(|s| s.mode.clone())
                        .or(Some(self.gradient.mode.clone())),
                    start_color: spec
                        .gradient
                        .as_ref()
                        .and_then(|s| s.start_color.clone())
                        .or(Some(self.gradient.start_color.clone())),
                    end_color: spec
                        .gradient
                        .as_ref()
                        .and_then(|s| s.end_color.clone())
                        .or(Some(self.gradient.end_color.clone())),
                }),
                pattern: Some(MonitorPatternConfig {
                    enabled: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.enabled)
                        .or(Some(self.pattern.enabled)),
                    source: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.source.clone())
                        .or(self.pattern.source.clone()),
                    sources: Some(MonitorPatternSourcesConfig {
                        top: spec
                            .pattern
                            .as_ref()
                            .and_then(|s| s.sources.as_ref())
                            .and_then(|sources| sources.top.clone())
                            .or(self.pattern.sources.top.clone()),
                        bottom: spec
                            .pattern
                            .as_ref()
                            .and_then(|s| s.sources.as_ref())
                            .and_then(|sources| sources.bottom.clone())
                            .or(self.pattern.sources.bottom.clone()),
                        left: spec
                            .pattern
                            .as_ref()
                            .and_then(|s| s.sources.as_ref())
                            .and_then(|sources| sources.left.clone())
                            .or(self.pattern.sources.left.clone()),
                        right: spec
                            .pattern
                            .as_ref()
                            .and_then(|s| s.sources.as_ref())
                            .and_then(|sources| sources.right.clone())
                            .or(self.pattern.sources.right.clone()),
                    }),
                    mode: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.mode.clone())
                        .or(Some(self.pattern.mode.clone())),
                    opacity: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.opacity)
                        .or(Some(self.pattern.opacity)),
                    color: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.color.clone())
                        .or(self.pattern.color.clone()),
                    blend: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.blend.clone())
                        .or(Some(self.pattern.blend.clone())),
                    seed: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.seed)
                        .or(Some(self.pattern.seed)),
                    transform: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.transform.clone())
                        .or(Some(self.pattern.transform.clone())),
                    repeat: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.repeat.clone())
                        .or(Some(self.pattern.repeat.clone())),
                    sides: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.sides.clone())
                        .or(Some(self.pattern.sides.clone())),
                    corners: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.corners.clone())
                        .or(Some(self.pattern.corners.clone())),
                    layout: spec
                        .pattern
                        .as_ref()
                        .and_then(|s| s.layout.clone())
                        .or(Some(self.pattern.layout.clone())),
                }),
                layer: match (&self.layer, &spec.layer) {
                    (g, Some(s)) => {
                        let mut merged = g.clone();
                        // For simplicity, we replace the whole LayerConfig if specified for monitor
                        // Alternatively, we could merge field by field if needed.
                        // Usually, users want to override the whole layer behavior per monitor.
                        merged.level = s.level.clone();
                        merged.exclusive_zone = s.exclusive_zone;
                        merged.keyboard_interactivity = s.keyboard_interactivity.clone();
                        merged.click_through = s.click_through;
                        merged.namespace = s.namespace.clone();
                        Some(merged)
                    }
                    (g, None) => Some(g.clone()),
                },
            }
        } else {
            global
        }
    }
}

fn load_value_with_includes(
    path: &Path,
    visited: &mut Vec<std::path::PathBuf>,
    loaded_files: &mut Vec<std::path::PathBuf>,
) -> Result<toml::Value, Box<dyn std::error::Error>> {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if visited.contains(&canonical) {
        return Err(format!("Circular include detected: {}", path.display()).into());
    }
    visited.push(canonical.clone());
    loaded_files.push(canonical.clone());

    let content = fs::read_to_string(path)?;
    let mut value: toml::Value = toml::from_str(&content)?;
    let mut merged = toml::Value::Table(toml::map::Map::new());

    let mut vars = HashMap::new();

    // 1. Process includes
    if let Some(includes) = value.get("include") {
        let include_paths: Vec<String> = match includes {
            toml::Value::String(s) => vec![s.clone()],
            toml::Value::Array(a) => a
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => vec![],
        };

        for inc in include_paths {
            let inc_path = path.parent().unwrap_or(Path::new("")).join(inc);
            match load_value_with_includes(&inc_path, visited, loaded_files) {
                Ok(inc_val) => {
                    if let Some(inc_vars) = inc_val.get("variables").and_then(|v| v.as_table()) {
                        for (k, v) in inc_vars {
                            vars.insert(k.clone(), v.clone());
                        }
                    }
                    merge_toml_values(&mut merged, inc_val);
                }
                Err(e) => {
                    return Err(format!("Failed to load included config {}: {}", inc_path.display(), e).into());
                }
            }
        }
    }

    merge_toml_values(&mut merged, value);
    value = merged;

    // 2. Extract local variables
    if let Some(local_vars) = value.get("variables").and_then(|v| v.as_table()) {
        for (k, v) in local_vars {
            vars.insert(k.clone(), v.clone());
        }
    }

    // 3. Expand variables
    expand_variables(&mut value, &vars);

    visited.pop();

    Ok(value)
}

fn merge_global_monitor_sections(value: &mut toml::Value) {
    let Some(root) = value.as_table_mut() else { return };
    let sections: Vec<(&str, toml::Value)> =
        ["frame_sides", "inner_shadow", "gradient", "pattern", "layer"]
            .into_iter()
            .filter_map(|key| root.get(key).cloned().map(|v| (key, v)))
            .collect();
    let Some(monitors) = root.get_mut("monitors").and_then(toml::Value::as_table_mut) else { return };
    for monitor in monitors.values_mut() {
        let Some(table) = monitor.as_table_mut() else { continue };
        for (key, global) in &sections {
            if let Some(local) = table.get_mut(*key) {
                let mut merged = global.clone();
                merge_toml_values(&mut merged, local.clone());
                *local = merged;
            }
        }
    }
}

fn merge_toml_values(base: &mut toml::Value, incoming: toml::Value) {
    match (base, incoming) {
        (toml::Value::Table(base_table), toml::Value::Table(incoming_table)) => {
            for (key, incoming_value) in incoming_table {
                if let Some(base_value) = base_table.get_mut(&key) {
                    merge_toml_values(base_value, incoming_value);
                } else {
                    base_table.insert(key, incoming_value);
                }
            }
        }
        (base_slot, incoming_value) => {
            *base_slot = incoming_value;
        }
    }
}

fn expand_variables(value: &mut toml::Value, vars: &HashMap<String, toml::Value>) {
    match value {
        toml::Value::String(s) => {
            // Full replacement
            if s.starts_with('$') && vars.contains_key(&s[1..]) {
                *value = vars[&s[1..]].clone();
            } else if s.starts_with("${")
                && s.ends_with('}')
                && vars.contains_key(&s[2..s.len() - 1])
            {
                *value = vars[&s[2..s.len() - 1]].clone();
            } else {
                // Substring interpolation
                let mut new_s = s.clone();
                for (k, v) in vars {
                    let replacement = match v {
                        toml::Value::String(v_str) => Some(v_str.clone()),
                        toml::Value::Integer(v_int) => Some(v_int.to_string()),
                        toml::Value::Float(v_float) => Some(v_float.to_string()),
                        toml::Value::Boolean(v_bool) => Some(v_bool.to_string()),
                        _ => None,
                    };

                    if let Some(r) = replacement {
                        new_s = new_s.replace(&format!("${}", k), &r);
                        new_s = new_s.replace(&format!("${{{}}}", k), &r);
                    }
                }
                *s = new_s;
            }
        }
        toml::Value::Array(arr) => {
            for v in arr.iter_mut() {
                expand_variables(v, vars);
            }
        }
        toml::Value::Table(tbl) => {
            for (_, v) in tbl.iter_mut() {
                expand_variables(v, vars);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env::temp_dir;
    use std::io::Write;
    use std::sync::{Mutex, OnceLock};

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    #[test]
    fn nested_monitor_overrides_keep_global_values() {
        let mut value: toml::Value = toml::from_str(r#"
[pattern.transform]
scale = 2.0
rotation = 10.0
[layer]
level = "overlay"
click_through = false
[monitors."DP-1".pattern.transform]
rotation = 45.0
[monitors."DP-1".layer]
level = "top"
"#).unwrap();
        merge_global_monitor_sections(&mut value);
        let cfg: Config = value.try_into().unwrap();
        let monitor = cfg.for_monitor("DP-1");
        let transform = monitor.pattern.unwrap().transform.unwrap();
        assert_eq!(transform.scale, 2.0);
        assert_eq!(transform.rotation, 45.0);
        let layer = monitor.layer.unwrap();
        assert_eq!(layer.level, "top");
        assert!(!layer.click_through);
    }

    #[test]
    fn missing_include_fails_config_load() {
        let dir = temp_dir().join(format!("rustbar-includes-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("rustbar.toml");
        fs::write(&path, "include = [\"missing.toml\"]\n").unwrap();
        let result = Config::try_load(&path);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_variable_expansion_and_include() {
        let dir = temp_dir();
        let main_path = dir.join("main.toml");
        let inc_path = dir.join("inc.toml");

        let mut inc_file = fs::File::create(&inc_path).unwrap();
        writeln!(inc_file, "[variables]\ncolor1 = \"#FF0000\"\nnum1 = 42").unwrap();

        let mut main_file = fs::File::create(&main_path).unwrap();
        let config_str = r##"
include = ["inc.toml"]

thickness = "$num1"
opacity = "$my_opacity"
color = "$my_color"
shadow_color = "${color1}AA"

[variables]
my_opacity = 0.5
my_color = "#00FF00"
"##;
        write!(main_file, "{}", config_str).unwrap();

        let config = Config::load(&main_path);

        assert_eq!(config.thickness, 42);
        assert_eq!(config.opacity, 0.5);
        assert_eq!(config.color, "#00FF00");
        assert_eq!(config.shadow_color, "#FF0000AA");

        // Clean up
    }

    #[test]
    fn test_variable_expansion_colors_toml() {
        let dir = temp_dir();
        let main_path = dir.join("main_issue.toml");
        let inc_path = dir.join("colors.toml");

        let colors_str = r##"
[variables]
background = "#191510"
foreground = "#ede6d8"
color1  = "#C0A57E"
"##;

        let main_str = r##"
include = ["colors.toml"]

color = "$color1"
opacity = 0.5
"##;

        let mut inc_file = fs::File::create(&inc_path).unwrap();
        write!(inc_file, "{}", colors_str).unwrap();

        let mut main_file = fs::File::create(&main_path).unwrap();
        write!(main_file, "{}", main_str).unwrap();

        let config = Config::load(&main_path);

        assert_eq!(config.color, "#C0A57E");
        assert!(
            config
                .loaded_files
                .contains(&main_path.canonicalize().unwrap())
        );
        assert!(
            config
                .loaded_files
                .contains(&inc_path.canonicalize().unwrap())
        );

        let _ = fs::remove_file(main_path);
        let _ = fs::remove_file(inc_path);
    }

    #[test]
    fn test_resolve_path_prefers_user_config_over_local() {
        let _guard = env_lock();
        let dir = temp_dir().join("rustbar-config-precedence-test");
        let xdg_dir = dir.join("xdg");
        let xdg_config = xdg_dir.join("rustbar").join("rustbar.toml");
        fs::create_dir_all(xdg_config.parent().unwrap()).unwrap();
        fs::write(&xdg_config, "color = \"#123456\"\n").unwrap();

        let local_path = std::env::current_dir().unwrap().join("rustbar.toml");
        let local_backup = if local_path.exists() {
            Some(fs::read_to_string(&local_path).unwrap())
        } else {
            None
        };
        fs::write(&local_path, "color = \"#abcdef\"\n").unwrap();

        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_rustbar_config = std::env::var("RUSTBAR_CONFIG").ok();

        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", &xdg_dir);
            std::env::remove_var("RUSTBAR_CONFIG");
        }

        let resolved = Config::resolve_path();
        assert_eq!(resolved, xdg_config);

        if let Some(prev) = old_xdg {
            unsafe { std::env::set_var("XDG_CONFIG_HOME", prev) };
        } else {
            unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        }

        if let Some(prev) = old_rustbar_config {
            unsafe { std::env::set_var("RUSTBAR_CONFIG", prev) };
        } else {
            unsafe { std::env::remove_var("RUSTBAR_CONFIG") };
        }

        if let Some(contents) = local_backup {
            fs::write(&local_path, contents).unwrap();
        } else {
            let _ = fs::remove_file(&local_path);
        }
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_apply_presets_to_config_value_merges_global_and_monitor_presets() {
        let mut value: toml::Value = toml::from_str(
            r##"
preset = "crt"
color = "#112233"

[presets.crt]
thickness = 28
radius = 40
shadow_width = 20.0

[presets.crt.pattern]
enabled = true
source = "grid.svg"
mode = "repeat"

[monitors."DP-1"]
preset = "thin"
opacity = 0.7

[presets.thin]
thickness = 6
radius = 10

[presets.thin.pattern]
enabled = true
source = "thin.svg"
mode = "diamond"

[monitors."DP-1".pattern]
opacity = 0.3
"##,
        )
        .unwrap();

        let warnings = apply_presets_to_config_value(&mut value);
        assert!(warnings.is_empty());

        let cfg: Config = value.try_into().unwrap();
        assert_eq!(cfg.thickness, 28);
        assert_eq!(cfg.radius, 40);
        assert_eq!(cfg.color, "#112233");
        assert_eq!(cfg.shadow_width, 20.0);
        assert!(cfg.pattern.enabled);
        assert_eq!(cfg.pattern.source.as_deref(), Some("grid.svg"));

        let monitor = cfg.for_monitor("DP-1");
        assert_eq!(monitor.thickness, Some(6));
        assert_eq!(monitor.radius, Some(10));
        assert_eq!(monitor.opacity, Some(0.7));
        assert_eq!(
            monitor.pattern.as_ref().and_then(|pattern| pattern.source.as_deref()),
            Some("thin.svg")
        );
        assert_eq!(
            monitor.pattern.as_ref().and_then(|pattern| pattern.mode.as_deref()),
            Some("diamond")
        );
        assert_eq!(
            monitor.pattern.as_ref().and_then(|pattern| pattern.opacity),
            Some(0.3)
        );
    }

    #[test]
    fn test_for_monitor_merges_frame_side_toggles() {
        let mut cfg = Config {
            frame_sides: FrameSidesConfig {
                top: false,
                bottom: true,
                left: true,
                right: true,
            },
            ..Config::default()
        };

        cfg.monitors.insert(
            "DP-1".to_string(),
            MonitorConfig {
                enabled: Some(true),
                thickness: None,
                thickness_top: None,
                thickness_bottom: None,
                thickness_left: None,
                thickness_right: None,
                frame_sides: Some(MonitorFrameSidesConfig {
                    top: None,
                    bottom: Some(false),
                    left: Some(false),
                    right: None,
                }),
                margin: None,
                margin_top: None,
                margin_bottom: None,
                margin_left: None,
                margin_right: None,
                offset_x: None,
                offset_y: None,
                radius: None,
                opacity: None,
                color: None,
                preset: None,
                gradient: None,
                shadow_width: None,
                shadow_color: None,
                highlight_width: None,
                highlight_color: None,
                inner_shadow: None,
                pattern: None,
                layer: None,
            },
        );

        let monitor = cfg.for_monitor("DP-1");
        let frame_sides = monitor.frame_sides.expect("merged frame sides");

        assert_eq!(frame_sides.top, Some(false));
        assert_eq!(frame_sides.bottom, Some(false));
        assert_eq!(frame_sides.left, Some(false));
        assert_eq!(frame_sides.right, Some(true));
    }

    #[test]
    fn test_for_monitor_merges_pattern_side_sources() {
        let mut cfg = Config::default();
        cfg.pattern.source = Some("fallback.svg".to_string());
        cfg.pattern.sources.top = Some("top.svg".to_string());
        cfg.pattern.sources.left = Some("left.svg".to_string());

        cfg.monitors.insert(
            "DP-1".to_string(),
            MonitorConfig {
                enabled: Some(true),
                thickness: None,
                thickness_top: None,
                thickness_bottom: None,
                thickness_left: None,
                thickness_right: None,
                frame_sides: None,
                margin: None,
                margin_top: None,
                margin_bottom: None,
                margin_left: None,
                margin_right: None,
                offset_x: None,
                offset_y: None,
                radius: None,
                opacity: None,
                color: None,
                preset: None,
                gradient: None,
                shadow_width: None,
                shadow_color: None,
                highlight_width: None,
                highlight_color: None,
                inner_shadow: None,
                pattern: Some(MonitorPatternConfig {
                    enabled: Some(true),
                    source: None,
                    sources: Some(MonitorPatternSourcesConfig {
                        top: None,
                        bottom: Some("bottom.svg".to_string()),
                        left: None,
                        right: Some("right.svg".to_string()),
                    }),
                    mode: None,
                    opacity: None,
                    color: None,
                    blend: None,
                    seed: None,
                    transform: None,
                    repeat: None,
                    sides: None,
                    corners: None,
                    layout: None,
                }),
                layer: None,
            },
        );

        let monitor = cfg.for_monitor("DP-1");
        let pattern = monitor.pattern.expect("merged pattern config");
        let sources = pattern.sources.expect("merged pattern sources");

        assert_eq!(pattern.source.as_deref(), Some("fallback.svg"));
        assert_eq!(sources.top.as_deref(), Some("top.svg"));
        assert_eq!(sources.bottom.as_deref(), Some("bottom.svg"));
        assert_eq!(sources.left.as_deref(), Some("left.svg"));
        assert_eq!(sources.right.as_deref(), Some("right.svg"));
    }

    #[test]
    fn test_apply_presets_to_config_value_warns_for_missing_preset() {
        let mut value: toml::Value = toml::from_str(
            r##"
preset = "missing"
color = "#abcdef"

[presets.valid]
thickness = 30
"##,
        )
        .unwrap();

        let warnings = apply_presets_to_config_value(&mut value);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("preset='missing'"));

        let cfg: Config = value.try_into().unwrap();
        assert_eq!(cfg.color, "#abcdef");
        assert_eq!(cfg.thickness, default_thickness());
    }

    #[test]
    fn test_try_load_applies_presets_before_deserialization() {
        let dir = temp_dir().join("rustbar-preset-load-test");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let config_path = dir.join("rustbar.toml");
        fs::write(
            &config_path,
            r##"
preset = "crt"

[presets.crt]
thickness = 26
radius = 34

[presets.crt.layer]
level = "top"

[monitors."DP-1"]
preset = "thin"

[presets.thin]
thickness = 8

[presets.thin.layer]
level = "overlay"
"##,
        )
        .unwrap();

        let cfg = Config::try_load(&config_path).unwrap();
        assert_eq!(cfg.thickness, 26);
        assert_eq!(cfg.radius, 34);
        assert_eq!(cfg.layer.level, "top");

        let monitor = cfg.for_monitor("DP-1");
        assert_eq!(monitor.thickness, Some(8));
        assert_eq!(
            monitor.layer.as_ref().map(|layer| layer.level.as_str()),
            Some("overlay")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_collect_monitor_validation_warnings_reports_silent_fallbacks() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: None,
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: None,
            opacity: Some(1.5),
            color: None,
            preset: None,
            shadow_width: None,
            shadow_color: None,
            highlight_width: None,
            highlight_color: None,
            inner_shadow: Some(MonitorInnerShadowConfig {
                enabled: Some(true),
                size: Some(10.0),
                opacity: Some(2.0),
                color: None,
                softness: Some(0.0),
                falloff: Some("weird".to_string()),
            }),
            gradient: Some(MonitorGradientConfig {
                enabled: Some(true),
                mode: Some("spiral".to_string()),
                start_color: Some("#111111".to_string()),
                end_color: Some("#222222".to_string()),
            }),
            pattern: Some(MonitorPatternConfig {
                enabled: Some(true),
                source: None,
                sources: None,
                mode: Some("spiral".to_string()),
                opacity: Some(-0.2),
                color: None,
                blend: Some("xor".to_string()),
                seed: None,
                transform: Some(PatternTransformConfig {
                    scale: 0.0,
                    rotation: 0.0,
                    flip_x: false,
                    flip_y: false,
                }),
                repeat: None,
                sides: None,
                corners: None,
                layout: None,
            }),
            layer: Some(LayerConfig {
                level: "ceiling".to_string(),
                exclusive_zone: 0,
                keyboard_interactivity: "sometimes".to_string(),
                click_through: true,
                namespace: "rustbar-frame".to_string(),
            }),
        };

        let warnings = collect_monitor_validation_warnings("DP-1", &config);

        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("layer.level='ceiling'"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("layer.keyboard_interactivity='sometimes'"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("pattern layer without setting pattern.source or pattern.sources.<side>"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("pattern.mode='spiral'"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("pattern.blend='xor'"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("inner_shadow.falloff='weird'"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("gradient.mode='spiral'"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("opacity=1.5"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("pattern.opacity=-0.2"))
        );
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("pattern.transform.scale=0"))
        );
    }

    #[test]
    fn test_collect_monitor_validation_warnings_reports_missing_pattern_asset() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: None,
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: None,
            opacity: None,
            color: None,
            preset: None,
            shadow_width: None,
            shadow_color: None,
            highlight_width: None,
            highlight_color: None,
            inner_shadow: None,
            gradient: None,
            pattern: Some(MonitorPatternConfig {
                enabled: Some(true),
                source: Some("definitely-missing-rustbar-test-asset".to_string()),
                sources: None,
                mode: Some("repeat".to_string()),
                opacity: Some(1.0),
                color: None,
                blend: Some("normal".to_string()),
                seed: None,
                transform: None,
                repeat: None,
                sides: None,
                corners: None,
                layout: None,
            }),
            layer: None,
        };

        let warnings = collect_monitor_validation_warnings("DP-1", &config);
        assert!(
            warnings
                .iter()
                .any(|warning| warning.contains("resolved SVG path")
                    && warning.contains("does not exist"))
        );
    }

    #[test]
    fn test_collect_monitor_validation_warnings_reports_missing_side_pattern_asset() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: None,
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: None,
            opacity: None,
            color: None,
            preset: None,
            shadow_width: None,
            shadow_color: None,
            highlight_width: None,
            highlight_color: None,
            inner_shadow: None,
            gradient: None,
            pattern: Some(MonitorPatternConfig {
                enabled: Some(true),
                source: None,
                sources: Some(MonitorPatternSourcesConfig {
                    top: Some("missing-top-pattern-asset".to_string()),
                    bottom: None,
                    left: None,
                    right: None,
                }),
                mode: Some("repeat".to_string()),
                opacity: Some(1.0),
                color: None,
                blend: Some("normal".to_string()),
                seed: None,
                transform: None,
                repeat: None,
                sides: None,
                corners: None,
                layout: None,
            }),
            layer: None,
        };

        let warnings = collect_monitor_validation_warnings("DP-1", &config);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("pattern.sources.top='missing-top-pattern-asset'")));
    }


    #[test]
    fn test_collect_monitor_validation_warnings_reports_missing_corner_pattern_asset() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: None,
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: None,
            opacity: None,
            color: None,
            preset: None,
            shadow_width: None,
            shadow_color: None,
            highlight_width: None,
            highlight_color: None,
            inner_shadow: None,
            gradient: None,
            pattern: Some(MonitorPatternConfig {
                enabled: Some(true),
                source: None,
                sources: None,
                mode: Some("repeat".to_string()),
                opacity: Some(1.0),
                color: None,
                blend: Some("normal".to_string()),
                seed: None,
                transform: None,
                repeat: None,
                sides: None,
                corners: Some(PatternCornersConfig {
                    avoid_corners: false,
                    corner_padding: 12.0,
                    top_left: Some("missing-corner-pattern-asset".to_string()),
                    top_right: None,
                    bottom_left: None,
                    bottom_right: None,
                }),
                layout: None,
            }),
            layer: None,
        };

        let warnings = collect_monitor_validation_warnings("DP-1", &config);
        assert!(warnings
            .iter()
            .any(|warning| warning.contains("pattern.corners.top_left='missing-corner-pattern-asset'")));
    }

    #[test]
    fn test_resolve_svg_path_uses_xdg_config_home() {
        let _guard = env_lock();
        let dir = temp_dir().join("rustbar-xdg-test");
        let svg_dir = dir.join("rustbar").join("svg");
        fs::create_dir_all(&svg_dir).unwrap();
        let svg_path = svg_dir.join("cross-svgrepo-com.svg");
        fs::write(&svg_path, r#"<svg xmlns="http://www.w3.org/2000/svg"/>"#).unwrap();

        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        unsafe { std::env::set_var("XDG_CONFIG_HOME", &dir) };
        let resolved = PatternConfig::resolve_svg_path("cross-svgrepo-com.svg");
        assert_eq!(resolved, svg_path);

        if let Some(prev) = old_xdg {
            unsafe { std::env::set_var("XDG_CONFIG_HOME", prev) };
        } else {
            unsafe { std::env::remove_var("XDG_CONFIG_HOME") };
        }

        let _ = fs::remove_file(&svg_path);
        let _ = fs::remove_dir_all(dir);
    }
}
