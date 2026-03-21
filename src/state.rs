use smithay_client_toolkit::{
    output::OutputState,
    registry::RegistryState,
    compositor::CompositorState,
    shm::{Shm, slot::{SlotPool, Buffer}},
    shell::{
        wlr_layer::{LayerShell, LayerSurface, Layer, KeyboardInteractivity},
        WaylandSurface,
    },
};
use wayland_client::{protocol::wl_shm, Proxy};
use std::collections::HashMap;

pub struct State {
    pub registry_state: RegistryState,
    pub compositor_state: CompositorState,
    pub output_state: OutputState,
    pub shm: Option<Shm>,
    pub layer_shell: Option<LayerShell>,
    pub config: crate::config::Config,
    pub loop_signal: calloop::LoopSignal,
    
    pub active_surfaces: Vec<LayerSurface>,
    pub dimensions: HashMap<u32, (u32, u32)>,
    pub pools: HashMap<u32, (SlotPool, Option<Buffer>)>,
    pub output_names: HashMap<u32, String>,
}

impl State {
    pub fn refresh_all(&mut self) {
        for layer in &self.active_surfaces {
            let id = layer.wl_surface().id().protocol_id();
            if let Some(&(width, height)) = self.dimensions.get(&id) {
                let name = self.output_names.get(&id).map(|s| s.as_str()).unwrap_or("default");
                let m_config = self.config.for_monitor(name);
                let pixels = crate::render::draw_frame(width, height, &m_config);
                
                if self.shm.is_some() {
                    if let Some((pool, active_buf)) = self.pools.get_mut(&id) {
                        let size = (width * height * 4) as usize;
                        if pool.resize(size).is_ok() {
                            if let Ok((buffer, canvas)) = pool.create_buffer(
                                width as i32, height as i32, width as i32 * 4, wl_shm::Format::Argb8888
                            ) {
                                canvas.copy_from_slice(&pixels);
                                layer.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
                                layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);
                                layer.wl_surface().commit();
                                *active_buf = Some(buffer);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_fullscreen(&mut self) {
        if let Ok(output) = std::process::Command::new("hyprctl").args(&["monitors", "-j"]).output() {
            if let Ok(monitors) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
                for m in monitors {
                    if let Some(name) = m["name"].as_str() {
                        let has_fullscreen = m["hasFullscreen"].as_bool().unwrap_or(false);
                        
                        for layer in &self.active_surfaces {
                            let id = layer.wl_surface().id().protocol_id();
                            if self.output_names.get(&id).map(|s| s.as_str()) == Some(name) {
                                let m_config = self.config.for_monitor(name);
                                let layer_config = m_config.layer.unwrap_or_default();
                                
                                if has_fullscreen {
                                    layer.set_layer(smithay_client_toolkit::shell::wlr_layer::Layer::Bottom);
                                } else {
                                    layer.set_layer(map_layer(&layer_config.level));
                                }
                                layer.commit();
                            }
                        }
                    }
                }
            }
        }
    }
}

pub fn map_layer(level: &str) -> Layer {
    match level.to_lowercase().as_str() {
        "background" => Layer::Background,
        "bottom" => Layer::Bottom,
        "top" => Layer::Top,
        "overlay" => Layer::Overlay,
        _ => Layer::Bottom,
    }
}

pub fn map_keyboard_interactivity(interactivity: &str) -> KeyboardInteractivity {
    match interactivity.to_lowercase().as_str() {
        "none" => KeyboardInteractivity::None,
        "on_demand" => KeyboardInteractivity::OnDemand,
        "exclusive" => KeyboardInteractivity::Exclusive,
        _ => KeyboardInteractivity::None,
    }
}
