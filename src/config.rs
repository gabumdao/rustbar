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
    pub monitors: HashMap<String, MonitorConfig>,
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
            monitors: HashMap::new(),
        }
    }
}

impl Config {
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str(&content) {
                Ok(cfg) => cfg,
                Err(e) => {
                    error!("Failed to parse config at {}: {}", path.display(), e);
                    Config::default()
                }
            },
            Err(e) => {
                info!("Could not read config at {} ({}), using defaults.", path.display(), e);
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
            }
        } else {
            global
        }
    }
}
