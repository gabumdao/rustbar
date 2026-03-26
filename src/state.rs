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
use std::collections::{HashMap, HashSet};
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
    pub output_scales: HashMap<u32, u32>,
    pub output_transforms: HashMap<u32, wl_output::Transform>,
    pub fullscreen_hidden: HashSet<u32>,
    pub watcher: Option<notify::RecommendedWatcher>,
}

impl State {
    pub(crate) fn apply_surface_output_state(
        &self,
        layer: &LayerSurface,
        scale: u32,
        transform: wl_output::Transform,
    ) -> (u32, wl_output::Transform) {
        let effective_scale = if layer.set_buffer_scale(scale).is_ok() {
            scale
        } else {
            1
        };
        let effective_transform = if layer.set_buffer_transform(transform).is_ok() {
            transform
        } else {
            wl_output::Transform::Normal
        };
        (effective_scale, effective_transform)
    }

    pub(crate) fn apply_layer_surface_config(
        &self,
        layer: &LayerSurface,
        qh: &QueueHandle<Self>,
        layer_config: &crate::config::LayerConfig,
        hidden: bool,
    ) {
        layer.set_layer(map_layer(&layer_config.level));
        layer.set_exclusive_zone(if hidden {
            0
        } else {
            layer_config.exclusive_zone
        });
        layer.set_keyboard_interactivity(if hidden {
            KeyboardInteractivity::None
        } else {
            map_keyboard_interactivity(&layer_config.keyboard_interactivity)
        });

        if hidden || layer_config.click_through {
            let region = self.compositor_state.wl_compositor().create_region(qh, ());
            layer.wl_surface().set_input_region(Some(&region));
            region.destroy();
        } else {
            layer.wl_surface().set_input_region(None);
        }

        layer.commit();
    }

    fn draw_surface(&mut self, surface_id: u32, transparent: bool) {
        let Some(layer) = self
            .active_surfaces
            .iter()
            .find(|layer| layer.wl_surface().id().protocol_id() == surface_id)
        else {
            return;
        };

        let Some(&(logical_width, logical_height)) = self.dimensions.get(&surface_id) else {
            return;
        };

        let requested_scale = self.output_scales.get(&surface_id).copied().unwrap_or(1);
        let requested_transform = self
            .output_transforms
            .get(&surface_id)
            .copied()
            .unwrap_or(wl_output::Transform::Normal);
        let (scale, transform) =
            self.apply_surface_output_state(layer, requested_scale, requested_transform);
        let (width, height) =
            scaled_buffer_dimensions(logical_width, logical_height, scale, transform);

        let pixels = if transparent {
            vec![0; (width * height * 4) as usize]
        } else {
            let name = self
                .output_names
                .get(&surface_id)
                .map(|s| s.as_str())
                .unwrap_or("default");
            let m_config = self.config.for_monitor(name);
            if m_config.enabled.unwrap_or(true) {
                crate::render::draw_frame(width, height, &m_config)
            } else {
                vec![0; (width * height * 4) as usize]
            }
        };

        if let Some(shm) = self.shm.as_mut() {
            let size = (width * height * 4) as usize;
            let (pool, active_buf) = self.pools.entry(surface_id).or_insert_with(|| {
                (
                    SlotPool::new(size, shm).expect("Failed to create slot pool"),
                    None,
                )
            });

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

    pub fn refresh_all(&mut self) {
        let surface_ids: Vec<u32> = self
            .active_surfaces
            .iter()
            .map(|layer| layer.wl_surface().id().protocol_id())
            .collect();

        for surface_id in surface_ids {
            let transparent = self.fullscreen_hidden.contains(&surface_id);
            self.draw_surface(surface_id, transparent);
        }
    }

    pub fn refresh_surface(&mut self, surface_id: u32) {
        let transparent = self.fullscreen_hidden.contains(&surface_id);
        self.draw_surface(surface_id, transparent);
    }

    pub fn surface_id_for_output_id(&self, output_id: u32) -> Option<u32> {
        self.output_ids
            .iter()
            .find_map(|(surface_id, stored_output_id)| {
                if *stored_output_id == output_id {
                    Some(*surface_id)
                } else {
                    None
                }
            })
    }

    pub fn surface_id_for_wl_surface(
        &self,
        surface: &wayland_client::protocol::wl_surface::WlSurface,
    ) -> Option<u32> {
        let surface_id = surface.id().protocol_id();
        self.active_surfaces
            .iter()
            .find(|layer| layer.wl_surface().id().protocol_id() == surface_id)
            .map(|_| surface_id)
    }

    pub fn reconcile_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        let output_id = output.id().protocol_id();
        let existing_surface_id = self.surface_id_for_output_id(output_id);

        let Some(info) = self.output_state.info(&output) else {
            return;
        };

        let name = info.name.unwrap_or_else(|| "Unknown".to_string());
        let scale = normalized_scale(info.scale_factor);
        let transform = info.transform;
        let m_config = self.config.for_monitor(&name);
        if !m_config.enabled.unwrap_or(true) {
            if let Some(surface_id) = existing_surface_id {
                self.remove_surface(surface_id);
            }
            return;
        }

        if existing_surface_id.is_none() {
            use smithay_client_toolkit::output::OutputHandler;
            self.new_output(conn, qh, output);
            return;
        }

        let surface_id = existing_surface_id.unwrap();
        self.output_names.insert(surface_id, name.clone());
        self.output_scales.insert(surface_id, scale);
        self.output_transforms.insert(surface_id, transform);

        if let Some(layer) = self
            .active_surfaces
            .iter()
            .find(|layer| layer.wl_surface().id().protocol_id() == surface_id)
        {
            let layer_config = m_config.layer.unwrap_or_default();
            let hidden = self.fullscreen_hidden.contains(&surface_id);
            self.apply_layer_surface_config(layer, qh, &layer_config, hidden);
        }

        self.refresh_surface(surface_id);
    }

    pub fn update_fullscreen(&mut self, qh: &QueueHandle<Self>) {
        if let Ok(output) = std::process::Command::new("hyprctl")
            .args(["monitors", "-j"])
            .output()
        {
            if let Ok(monitors) = serde_json::from_slice::<Vec<serde_json::Value>>(&output.stdout) {
                let mut fullscreen_outputs = HashSet::new();
                for m in monitors {
                    if m["hasFullscreen"].as_bool().unwrap_or(false) {
                        if let Some(name) = m["name"].as_str() {
                            fullscreen_outputs.insert(name.to_string());
                        }
                    }
                }

                let updates: Vec<(u32, bool, crate::config::LayerConfig)> = self
                    .active_surfaces
                    .iter()
                    .filter_map(|layer| {
                        let id = layer.wl_surface().id().protocol_id();
                        let name = self.output_names.get(&id)?;
                        let m_config = self.config.for_monitor(name);
                        Some((
                            id,
                            fullscreen_outputs.contains(name),
                            m_config.layer.unwrap_or_default(),
                        ))
                    })
                    .collect();

                let mut needs_refresh = false;
                for (surface_id, hidden, layer_config) in updates {
                    let was_hidden = self.fullscreen_hidden.contains(&surface_id);
                    if hidden {
                        self.fullscreen_hidden.insert(surface_id);
                    } else {
                        self.fullscreen_hidden.remove(&surface_id);
                    }

                    if let Some(layer) = self
                        .active_surfaces
                        .iter()
                        .find(|layer| layer.wl_surface().id().protocol_id() == surface_id)
                    {
                        self.apply_layer_surface_config(layer, qh, &layer_config, hidden);
                    }

                    if was_hidden != hidden {
                        needs_refresh = true;
                    }
                }

                if needs_refresh {
                    self.refresh_all();
                }
            }
        }
    }

    pub fn forget_surface(&mut self, surface_id: u32) {
        self.dimensions.remove(&surface_id);
        self.pools.remove(&surface_id);
        self.output_names.remove(&surface_id);
        self.output_ids.remove(&surface_id);
        self.output_scales.remove(&surface_id);
        self.output_transforms.remove(&surface_id);
        self.fullscreen_hidden.remove(&surface_id);
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
        let surfaces_to_remove: Vec<u32> = self
            .active_surfaces
            .iter()
            .filter_map(|layer| {
                let surface_id = layer.wl_surface().id().protocol_id();
                let name = self.output_names.get(&surface_id)?;
                if self.config.for_monitor(name).enabled.unwrap_or(true) {
                    None
                } else {
                    Some(surface_id)
                }
            })
            .collect();

        for surface_id in surfaces_to_remove {
            self.remove_surface(surface_id);
        }

        for layer in &self.active_surfaces {
            let surface_id = layer.wl_surface().id().protocol_id();
            if let Some(name) = self.output_names.get(&surface_id) {
                let m_config = self.config.for_monitor(name);
                let layer_config = m_config.layer.unwrap_or_default();
                let hidden = self.fullscreen_hidden.contains(&surface_id);
                self.apply_layer_surface_config(layer, qh, &layer_config, hidden);
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

pub fn normalized_scale(scale_factor: i32) -> u32 {
    scale_factor.max(1) as u32
}

pub fn transform_swaps_dimensions(transform: wl_output::Transform) -> bool {
    matches!(
        transform,
        wl_output::Transform::_90
            | wl_output::Transform::_270
            | wl_output::Transform::Flipped90
            | wl_output::Transform::Flipped270
    )
}

pub fn scaled_buffer_dimensions(
    logical_width: u32,
    logical_height: u32,
    scale: u32,
    transform: wl_output::Transform,
) -> (u32, u32) {
    let (width, height) = if transform_swaps_dimensions(transform) {
        (logical_height, logical_width)
    } else {
        (logical_width, logical_height)
    };
    (width.saturating_mul(scale), height.saturating_mul(scale))
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

pub fn effective_namespace(namespace: &str) -> String {
    let trimmed = namespace.trim();
    if trimmed.is_empty() {
        "rustbar-frame".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_layer_falls_back_to_bottom() {
        assert_eq!(map_layer("background"), Layer::Background);
        assert_eq!(map_layer("overlay"), Layer::Overlay);
        assert_eq!(map_layer("invalid"), Layer::Bottom);
    }

    #[test]
    fn map_keyboard_interactivity_falls_back_to_none() {
        assert_eq!(
            map_keyboard_interactivity("none"),
            KeyboardInteractivity::None
        );
        assert_eq!(
            map_keyboard_interactivity("exclusive"),
            KeyboardInteractivity::Exclusive
        );
        assert_eq!(
            map_keyboard_interactivity("invalid"),
            KeyboardInteractivity::None
        );
    }

    #[test]
    fn effective_namespace_uses_default_for_blank_values() {
        assert_eq!(effective_namespace("rustbar-custom"), "rustbar-custom");
        assert_eq!(effective_namespace("   "), "rustbar-frame");
        assert_eq!(effective_namespace(""), "rustbar-frame");
    }

    #[test]
    fn normalized_scale_clamps_to_one() {
        assert_eq!(normalized_scale(0), 1);
        assert_eq!(normalized_scale(-5), 1);
        assert_eq!(normalized_scale(2), 2);
    }

    #[test]
    fn scaled_buffer_dimensions_swap_for_quarter_turn_transforms() {
        assert_eq!(
            scaled_buffer_dimensions(1920, 1080, 2, wl_output::Transform::Normal),
            (3840, 2160)
        );
        assert_eq!(
            scaled_buffer_dimensions(1920, 1080, 2, wl_output::Transform::_90),
            (2160, 3840)
        );
        assert_eq!(
            scaled_buffer_dimensions(1920, 1080, 1, wl_output::Transform::Flipped270),
            (1080, 1920)
        );
    }
}
