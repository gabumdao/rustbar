use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use log::{error, info};

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct Config {
    #[serde(default = "default_thickness")]
    pub thickness: u32,
    #[serde(default = "default_radius")]
    pub radius: u32,
    #[serde(default = "default_opacity")]
    pub opacity: f32,
    #[serde(default = "default_color")]
    pub color: String,
    #[serde(default)]
    pub preset: Option<String>,
    #[serde(default)]
    pub monitors: HashMap<String, MonitorConfig>,
}

#[derive(Debug, Deserialize, Clone, PartialEq)]
pub struct MonitorConfig {
    pub enabled: Option<bool>,
    pub thickness: Option<u32>,
    pub radius: Option<u32>,
    pub opacity: Option<f32>,
    pub color: Option<String>,
    pub preset: Option<String>,
}

fn default_thickness() -> u32 { 16 }
fn default_radius() -> u32 { 24 }
fn default_opacity() -> f32 { 1.0 }
fn default_color() -> String { "#000000".to_string() }

impl Default for Config {
    fn default() -> Self {
        Self {
            thickness: default_thickness(),
            radius: default_radius(),
            opacity: default_opacity(),
            color: default_color(),
            preset: None,
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
            radius: Some(self.radius),
            opacity: Some(self.opacity),
            color: Some(self.color.clone()),
            preset: self.preset.clone(),
        };

        if let Some(spec) = self.monitors.get(name) {
            MonitorConfig {
                enabled: spec.enabled.or(global.enabled),
                thickness: spec.thickness.or(global.thickness),
                radius: spec.radius.or(global.radius),
                opacity: spec.opacity.or(global.opacity),
                color: spec.color.clone().or(global.color),
                preset: spec.preset.clone().or(global.preset),
            }
        } else {
            global
        }
    }
}
