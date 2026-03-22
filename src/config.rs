use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use log::{error, info};

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
    pub layer: LayerConfig,

    #[serde(default)]
    pub monitors: HashMap<String, MonitorConfig>,

    #[serde(skip)]
    pub loaded_files: Vec<std::path::PathBuf>,
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

fn default_layer_level() -> String { "bottom".to_string() }
fn default_exclusive_zone() -> i32 { 0 }
fn default_keyboard_interactivity() -> String { "none".to_string() }
fn default_click_through() -> bool { true }
fn default_namespace() -> String { "rustbar-frame".to_string() }

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
    pub layer: Option<LayerConfig>,
}

fn default_thickness() -> u32 { 16 }
fn default_radius() -> u32 { 24 }
fn default_opacity() -> f32 { 1.0 }
fn default_color() -> String { "#000000".to_string() }
fn default_shadow_width() -> f32 { 12.0 }
fn default_shadow_color() -> String { "#000000AA".to_string() }
fn default_highlight_width() -> f32 { 3.0 }
fn default_highlight_color() -> String { "#FFFFFF22".to_string() }

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
            layer: LayerConfig::default(),
            monitors: HashMap::new(),
            loaded_files: Vec::new(),
        }
    }
}

impl Config {
    pub fn load_default() -> Self {
        Self::load(&Self::resolve_path())
    }

    pub fn resolve_path() -> std::path::PathBuf {
        let local = std::path::Path::new("rustbar.toml");
        if local.exists() {
            return local.to_path_buf();
        }
        if let Ok(xdg_config_home) = std::env::var("XDG_CONFIG_HOME") {
            let xdg_path = std::path::Path::new(&xdg_config_home).join("rustbar").join("rustbar.toml");
            if xdg_path.exists() {
                return xdg_path;
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let home_path = std::path::Path::new(&home).join(".config").join("rustbar").join("rustbar.toml");
            if home_path.exists() {
                return home_path;
            }
        }
        // Fallback to local
        std::path::PathBuf::from("rustbar.toml")
    }

    pub fn load(path: &Path) -> Self {
        let mut loaded_files = Vec::new();
        let config_value = match load_value_with_includes(path, &mut Vec::new(), &mut loaded_files) {
            Ok(v) => v,
            Err(e) => {
                info!("Could not read or parse config at {}: {}", path.display(), e);
                return Config::default();
            }
        };

        match config_value.try_into::<Config>() {
            Ok(mut cfg) => {
                cfg.loaded_files = loaded_files;
                cfg
            }
            Err(e) => {
                error!("Failed to parse config at {}: {}", path.display(), e);
                Config::default()
            }
        }
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
                }
                Err(e) => {
                    error!("Failed to load included config {}: {}", inc_path.display(), e);
                }
            }
        }
    }

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

fn expand_variables(value: &mut toml::Value, vars: &HashMap<String, toml::Value>) {
    match value {
        toml::Value::String(s) => {
            // Full replacement
            if s.starts_with('$') && vars.contains_key(&s[1..]) {
                *value = vars[&s[1..]].clone();
            } else if s.starts_with("${") && s.ends_with('}') && vars.contains_key(&s[2..s.len() - 1]) {
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
    use std::io::Write;
    use std::env::temp_dir;

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
        assert!(config.loaded_files.contains(&main_path.canonicalize().unwrap()));
        assert!(config.loaded_files.contains(&inc_path.canonicalize().unwrap()));

        let _ = fs::remove_file(main_path);
        let _ = fs::remove_file(inc_path);
    }
}

