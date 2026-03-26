use log::{error, info};
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

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct PatternCornersConfig {
    #[serde(default = "default_pattern_avoid_corners")]
    pub avoid_corners: bool,
    #[serde(default = "default_pattern_corner_padding")]
    pub corner_padding: f32,
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
    pub radius: Option<u32>,
    pub opacity: Option<f32>,
    pub color: Option<String>,
    pub preset: Option<String>,
    pub shadow_width: Option<f32>,
    pub shadow_color: Option<String>,
    pub highlight_width: Option<f32>,
    pub highlight_color: Option<String>,
    pub inner_shadow: Option<MonitorInnerShadowConfig>,
    pub pattern: Option<MonitorPatternConfig>,
    pub layer: Option<LayerConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorPatternConfig {
    pub enabled: Option<bool>,
    pub source: Option<String>,
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
            radius: default_radius(),
            opacity: default_opacity(),
            color: default_color(),
            preset: None,
            shadow_width: default_shadow_width(),
            shadow_color: default_shadow_color(),
            highlight_width: default_highlight_width(),
            highlight_color: default_highlight_color(),
            inner_shadow: InnerShadowConfig::default(),
            pattern: PatternConfig::default(),
            layer: LayerConfig::default(),
            monitors: HashMap::new(),
            loaded_files: Vec::new(),
        }
    }
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

        let mut cfg: Config = config_value
            .try_into()
            .map_err(|e| format!("{}: {}", path.display(), e))?;
        cfg.loaded_files = loaded_files;
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
            pattern: Some(MonitorPatternConfig {
                enabled: Some(self.pattern.enabled),
                source: self.pattern.source.clone(),
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
                    error!(
                        "Failed to load included config {}: {}",
                        inc_path.display(),
                        e
                    );
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
