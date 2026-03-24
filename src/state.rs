use smithay_client_toolkit::{
    compositor::CompositorState,
    output::OutputState,
    registry::RegistryState,
    shell::{
        WaylandSurface,
        wlr_layer::{KeyboardInteractivity, Layer, LayerShell, LayerSurface},
    },
    shm::{
        Shm,
        slot::{Buffer, SlotPool},
    },
};
use std::collections::HashMap;
use wayland_client::{
    Connection, Proxy, QueueHandle,
    protocol::{wl_output, wl_shm},
};

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
    pub output_ids: HashMap<u32, u32>,
    pub watcher: Option<notify::RecommendedWatcher>,
}

impl State {
    pub fn refresh_all(&mut self) {
        for layer in &self.active_surfaces {
            let id = layer.wl_surface().id().protocol_id();
            if let Some(&(width, height)) = self.dimensions.get(&id) {
                let name = self
                    .output_names
                    .get(&id)
                    .map(|s| s.as_str())
                    .unwrap_or("default");
                let m_config = self.config.for_monitor(name);
                let pixels = if m_config.enabled.unwrap_or(true) {
                    crate::render::draw_frame(width, height, &m_config)
                } else {
                    vec![0; (width * height * 4) as usize]
                };

                if let Some((pool, active_buf)) = self.pools.get_mut(&id) {
                    let size = (width * height * 4) as usize;
                    if pool.resize(size).is_ok() {
                        if let Ok((buffer, canvas)) = pool.create_buffer(
                            width as i32,
                            height as i32,
                            width as i32 * 4,
                            wl_shm::Format::Argb8888,
                        ) {
                            canvas.copy_from_slice(&pixels);
                            layer.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
                            layer
                                .wl_surface()
                                .damage_buffer(0, 0, width as i32, height as i32);
                            layer.wl_surface().commit();
                            *active_buf = Some(buffer);
                        }
                    }
                }
            }
        }
    }

    pub fn update_fullscreen(&mut self) {
        if let Ok(output) = std::process::Command::new("hyprctl")
            .args(["monitors", "-j"])
            .output()
        {
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
                                    layer.set_layer(Layer::Bottom);
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

    pub fn forget_surface(&mut self, surface_id: u32) {
        self.dimensions.remove(&surface_id);
        self.pools.remove(&surface_id);
        self.output_names.remove(&surface_id);
        self.output_ids.remove(&surface_id);
    }

    pub fn remove_surface(&mut self, surface_id: u32) {
        if let Some(index) = self
            .active_surfaces
            .iter()
            .position(|surface| surface.wl_surface().id().protocol_id() == surface_id)
        {
            let layer = self.active_surfaces.remove(index);
            layer.wl_surface().destroy();
        }
        self.forget_surface(surface_id);
    }

    pub fn sync_outputs(&mut self, conn: &Connection, qh: &QueueHandle<Self>) {
        for layer in &self.active_surfaces {
            let surface_id = layer.wl_surface().id().protocol_id();
            if let Some(name) = self.output_names.get(&surface_id) {
                let m_config = self.config.for_monitor(name);
                let layer_config = m_config.layer.unwrap_or_default();

                layer.set_layer(map_layer(&layer_config.level));
                layer.set_exclusive_zone(layer_config.exclusive_zone);
                layer.set_keyboard_interactivity(map_keyboard_interactivity(
                    &layer_config.keyboard_interactivity,
                ));

                if layer_config.click_through {
                    let region = self.compositor_state.wl_compositor().create_region(qh, ());
                    layer.wl_surface().set_input_region(Some(&region));
                    region.destroy();
                } else {
                    layer.wl_surface().set_input_region(None);
                }

                layer.commit();
            }
        }

        let existing_outputs: std::collections::HashSet<String> =
            self.output_names.values().cloned().collect();
        let outputs: Vec<wl_output::WlOutput> = self.output_state.outputs().collect();
        for output in outputs {
            if let Some(info) = self.output_state.info(&output) {
                let name = info.name.unwrap_or_else(|| "Unknown".to_string());
                if !existing_outputs.contains(&name)
                    && self.config.for_monitor(&name).enabled.unwrap_or(true)
                {
                    use smithay_client_toolkit::output::OutputHandler;
                    self.new_output(conn, qh, output);
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
