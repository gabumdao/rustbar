use log::debug;
use serde_json::json;
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
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};
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
    pub render_caches: HashMap<u32, crate::render::FrameRenderCache>,
    pub last_reload_at: Option<std::time::Instant>,
    pub fullscreen_hidden: HashSet<u32>,
    pub manually_hidden: bool,
    pub manually_hidden_outputs: HashSet<String>,
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
        let Some(size) = checked_frame_buffer_size(width, height) else {
            log::warn!("Skipping invalid or oversized rustbar buffer {}x{}", width, height);
            return;
        };
        let output_name = self
            .output_names
            .get(&surface_id)
            .cloned()
            .unwrap_or_else(|| "default".to_string());

        let render_start = Instant::now();
        let pixels = if transparent {
            vec![0; size]
        } else {
            let m_config = self.config.for_monitor(&output_name);
            if m_config.enabled.unwrap_or(true) {
                self
                    .render_caches
                    .entry(surface_id)
                    .or_default()
                    .draw_frame(width, height, &m_config)
            } else {
                vec![0; size]
            }
        };
        let render_elapsed = render_start.elapsed();
        if pixels.len() != size {
            log::warn!(
                "Skipping buffer update for {}: renderer returned {} bytes, expected {}",
                output_name, pixels.len(), size
            );
            return;
        }

        let upload_start = Instant::now();
        let mut committed = false;
        if let Some(shm) = self.shm.as_mut() {
            if !self.pools.contains_key(&surface_id) {
                match SlotPool::new(size, shm) {
                    Ok(pool) => {
                        self.pools.insert(surface_id, (pool, None));
                    }
                    Err(err) => {
                        log::warn!("Unable to allocate SHM slot pool for {}: {}", output_name, err);
                        return;
                    }
                }
            }
            let Some((pool, active_buf)) = self.pools.get_mut(&surface_id) else {
                return;
            };
            if pool.resize(size).is_ok() {
                if let Ok((buffer, canvas)) = pool.create_buffer(
                    width as i32,
                    height as i32,
                    width as i32 * 4,
                    wl_shm::Format::Argb8888,
                ) {
                    canvas.copy_from_slice(&pixels);
                    if buffer.attach_to(layer.wl_surface()).is_ok() {
                        layer
                            .wl_surface()
                            .damage_buffer(0, 0, width as i32, height as i32);
                        layer.wl_surface().commit();
                        *active_buf = Some(buffer);
                        committed = true;
                    }
                }
            }
        }
        let upload_elapsed = upload_start.elapsed();

        debug!(
            "redraw output={} surface_id={} logical={}x{} buffer={}x{} scale={} transform={:?} transparent={} render_ms={:.3} upload_ms={:.3} committed={}",
            output_name,
            surface_id,
            logical_width,
            logical_height,
            width,
            height,
            scale,
            transform,
            transparent,
            render_elapsed.as_secs_f64() * 1000.0,
            upload_elapsed.as_secs_f64() * 1000.0,
            committed,
        );
    }

    pub fn surface_is_hidden(&self, surface_id: u32) -> bool {
        let output_hidden = self
            .output_names
            .get(&surface_id)
            .map(|name| self.manually_hidden_outputs.contains(name))
            .unwrap_or(false);
        effective_visibility_hidden(
            self.manually_hidden,
            output_hidden,
            self.fullscreen_hidden.contains(&surface_id),
        )
    }

    pub fn set_manual_hidden(&mut self, hidden: bool, qh: &QueueHandle<Self>) {
        if self.manually_hidden == hidden {
            return;
        }
        self.manually_hidden = hidden;

        for layer in &self.active_surfaces {
            let surface_id = layer.wl_surface().id().protocol_id();
            if let Some(name) = self.output_names.get(&surface_id) {
                let m_config = self.config.for_monitor(name);
                let layer_config = m_config.layer.unwrap_or_default();
                let effective_hidden = self.surface_is_hidden(surface_id);
                self.apply_layer_surface_config(layer, qh, &layer_config, effective_hidden);
            }
        }

        self.refresh_all();
    }

    pub fn set_output_hidden(&mut self, output_name: &str, hidden: bool, qh: &QueueHandle<Self>) {
        let normalized = output_name.trim();
        if normalized.is_empty() {
            return;
        }

        if hidden {
            self.manually_hidden_outputs.insert(normalized.to_string());
        } else {
            self.manually_hidden_outputs.remove(normalized);
        }

        for layer in &self.active_surfaces {
            let surface_id = layer.wl_surface().id().protocol_id();
            if self.output_names.get(&surface_id).map(String::as_str) == Some(normalized) {
                let m_config = self.config.for_monitor(normalized);
                let layer_config = m_config.layer.unwrap_or_default();
                let effective_hidden = self.surface_is_hidden(surface_id);
                self.apply_layer_surface_config(layer, qh, &layer_config, effective_hidden);
                self.refresh_surface(surface_id);
                break;
            }
        }
    }

    pub fn toggle_output_hidden(&mut self, output_name: &str, qh: &QueueHandle<Self>) {
        let normalized = output_name.trim();
        if normalized.is_empty() {
            return;
        }
        let hidden = !self.manually_hidden_outputs.contains(normalized);
        self.set_output_hidden(normalized, hidden, qh);
    }

    pub fn refresh_all(&mut self) {
        let surface_ids: Vec<u32> = self
            .active_surfaces
            .iter()
            .map(|layer| layer.wl_surface().id().protocol_id())
            .collect();

        for surface_id in surface_ids {
            let transparent = self.surface_is_hidden(surface_id);
            self.draw_surface(surface_id, transparent);
        }
    }

    pub fn refresh_surface(&mut self, surface_id: u32) {
        let transparent = self.surface_is_hidden(surface_id);
        self.draw_surface(surface_id, transparent);
    }

    pub fn status_summary(&self) -> String {
        let mut outputs: Vec<String> = self.output_names.values().cloned().collect();
        outputs.sort();
        outputs.dedup();

        let mut hidden_outputs: Vec<String> = self
            .fullscreen_hidden
            .iter()
            .filter_map(|surface_id| self.output_names.get(surface_id).cloned())
            .collect();
        hidden_outputs.sort();
        hidden_outputs.dedup();

        let mut manually_hidden_outputs: Vec<String> = self.manually_hidden_outputs.iter().cloned().collect();
        manually_hidden_outputs.sort();

        format!(
            "status=ok
config_path={}
loaded_files={}
active_surfaces={}
outputs={}
fullscreen_hidden={}
fullscreen_outputs={}
manually_hidden={}
manually_hidden_outputs={}
",
            crate::config::Config::resolve_path().display(),
            self.config.loaded_files.len(),
            self.active_surfaces.len(),
            if outputs.is_empty() {
                "none".to_string()
            } else {
                outputs.join(",")
            },
            self.fullscreen_hidden.len(),
            if hidden_outputs.is_empty() {
                "none".to_string()
            } else {
                hidden_outputs.join(",")
            },
            self.manually_hidden,
            if manually_hidden_outputs.is_empty() {
                "none".to_string()
            } else {
                manually_hidden_outputs.join(",")
            },
        )
    }

    pub fn status_summary_for_output(&self, output_name: &str) -> String {
        let normalized = output_name.trim();
        if normalized.is_empty() {
            return "error: output name cannot be blank
".to_string();
        }

        let Some(surface_id) = self
            .output_names
            .iter()
            .find_map(|(surface_id, name)| if name == normalized { Some(*surface_id) } else { None })
        else {
            return format!("status=missing
output={}
", normalized);
        };

        let (logical_width, logical_height) = self.dimensions.get(&surface_id).copied().unwrap_or((0, 0));
        let scale = self.output_scales.get(&surface_id).copied().unwrap_or(1);
        let transform = self
            .output_transforms
            .get(&surface_id)
            .copied()
            .unwrap_or(wl_output::Transform::Normal);
        let fullscreen_hidden = self.fullscreen_hidden.contains(&surface_id);
        let manually_hidden = self.manually_hidden_outputs.contains(normalized);
        let effective_hidden = self.surface_is_hidden(surface_id);

        format!(
            "status=ok
output={}
surface_id={}
logical_size={}x{}
scale={}
transform={:?}
fullscreen_hidden={}
manually_hidden={}
globally_hidden={}
effective_hidden={}
",
            normalized,
            surface_id,
            logical_width,
            logical_height,
            scale,
            transform,
            fullscreen_hidden,
            manually_hidden,
            self.manually_hidden,
            effective_hidden,
        )
    }

    pub fn list_outputs_summary(&self) -> String {
        let mut outputs: Vec<(String, u32)> = self
            .output_names
            .iter()
            .map(|(surface_id, name)| (name.clone(), *surface_id))
            .collect();
        outputs.sort_by(|a, b| a.0.cmp(&b.0));
        outputs.dedup_by(|a, b| a.0 == b.0);

        let mut lines = vec![String::from("status=ok")];
        if outputs.is_empty() {
            lines.push(String::from("outputs=none"));
        } else {
            lines.push(format!(
                "outputs={}",
                outputs
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ));
            for (name, surface_id) in outputs {
                let (logical_width, logical_height) =
                    self.dimensions.get(&surface_id).copied().unwrap_or((0, 0));
                let scale = self.output_scales.get(&surface_id).copied().unwrap_or(1);
                let transform = self
                    .output_transforms
                    .get(&surface_id)
                    .copied()
                    .unwrap_or(wl_output::Transform::Normal);
                lines.push(format!(
                    "output={} surface_id={} logical_size={}x{} scale={} transform={:?} fullscreen_hidden={} manually_hidden={} effective_hidden={}",
                    name,
                    surface_id,
                    logical_width,
                    logical_height,
                    scale,
                    transform,
                    self.fullscreen_hidden.contains(&surface_id),
                    self.manually_hidden_outputs.contains(&name),
                    self.surface_is_hidden(surface_id),
                ));
            }
        }
        lines.push(String::new());
        lines.join("
")
    }

    fn output_snapshot(&self, output_name: &str, surface_id: u32) -> serde_json::Value {
        let (logical_width, logical_height) = self.dimensions.get(&surface_id).copied().unwrap_or((0, 0));
        let scale = self.output_scales.get(&surface_id).copied().unwrap_or(1);
        let transform = self
            .output_transforms
            .get(&surface_id)
            .copied()
            .unwrap_or(wl_output::Transform::Normal);
        let fullscreen_hidden = self.fullscreen_hidden.contains(&surface_id);
        let manually_hidden = self.manually_hidden_outputs.contains(output_name);
        let effective_hidden = self.surface_is_hidden(surface_id);

        json!({
            "output": output_name,
            "surface_id": surface_id,
            "logical_size": {
                "width": logical_width,
                "height": logical_height,
            },
            "scale": scale,
            "transform": format!("{:?}", transform),
            "fullscreen_hidden": fullscreen_hidden,
            "manually_hidden": manually_hidden,
            "globally_hidden": self.manually_hidden,
            "effective_hidden": effective_hidden,
        })
    }

    pub fn status_summary_json(&self) -> String {
        let mut outputs: Vec<String> = self.output_names.values().cloned().collect();
        outputs.sort();
        outputs.dedup();

        let mut fullscreen_outputs: Vec<String> = self
            .fullscreen_hidden
            .iter()
            .filter_map(|surface_id| self.output_names.get(surface_id).cloned())
            .collect();
        fullscreen_outputs.sort();
        fullscreen_outputs.dedup();

        let mut manually_hidden_outputs: Vec<String> = self.manually_hidden_outputs.iter().cloned().collect();
        manually_hidden_outputs.sort();

        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "config_path": crate::config::Config::resolve_path(),
            "loaded_files": self.config.loaded_files.len(),
            "active_surfaces": self.active_surfaces.len(),
            "outputs": outputs,
            "fullscreen_hidden": self.fullscreen_hidden.len(),
            "fullscreen_outputs": fullscreen_outputs,
            "manually_hidden": self.manually_hidden,
            "manually_hidden_outputs": manually_hidden_outputs,
        }))
        .map(|mut s| {
            s.push('\n');
            s
        })
        .unwrap_or_else(|_| "{\"status\":\"error\",\"message\":\"failed to serialize status\"}\n".to_string())
    }

    pub fn status_summary_for_output_json(&self, output_name: &str) -> String {
        let normalized = output_name.trim();
        if normalized.is_empty() {
            return "{\"status\":\"error\",\"message\":\"output name cannot be blank\"}\n".to_string();
        }

        let Some(surface_id) = self
            .output_names
            .iter()
            .find_map(|(surface_id, name)| if name == normalized { Some(*surface_id) } else { None })
        else {
            return serde_json::to_string_pretty(&json!({
                "status": "missing",
                "output": normalized,
            }))
            .map(|mut s| {
                s.push('\n');
                s
            })
            .unwrap_or_else(|_| "{\"status\":\"error\",\"message\":\"failed to serialize output status\"}\n".to_string());
        };

        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "details": self.output_snapshot(normalized, surface_id),
        }))
        .map(|mut s| {
            s.push('\n');
            s
        })
        .unwrap_or_else(|_| "{\"status\":\"error\",\"message\":\"failed to serialize output status\"}\n".to_string())
    }

    pub fn list_outputs_summary_json(&self) -> String {
        let mut outputs: Vec<(String, u32)> = self
            .output_names
            .iter()
            .map(|(surface_id, name)| (name.clone(), *surface_id))
            .collect();
        outputs.sort_by(|a, b| a.0.cmp(&b.0));
        outputs.dedup_by(|a, b| a.0 == b.0);

        let output_details: Vec<serde_json::Value> = outputs
            .into_iter()
            .map(|(name, surface_id)| self.output_snapshot(&name, surface_id))
            .collect();

        serde_json::to_string_pretty(&json!({
            "status": "ok",
            "outputs": output_details,
        }))
        .map(|mut s| {
            s.push('\n');
            s
        })
        .unwrap_or_else(|_| "{\"status\":\"error\",\"message\":\"failed to serialize outputs list\"}\n".to_string())
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
            let hidden = self.surface_is_hidden(surface_id);
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
                    let was_effectively_hidden = self.surface_is_hidden(surface_id);
                    if hidden {
                        self.fullscreen_hidden.insert(surface_id);
                    } else {
                        self.fullscreen_hidden.remove(&surface_id);
                    }
                    let effectively_hidden = self.surface_is_hidden(surface_id);

                    if was_effectively_hidden != effectively_hidden {
                        if let Some(layer) = self
                            .active_surfaces
                            .iter()
                            .find(|layer| layer.wl_surface().id().protocol_id() == surface_id)
                        {
                            self.apply_layer_surface_config(layer, qh, &layer_config, effectively_hidden);
                        }
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
        self.render_caches.remove(&surface_id);
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
                let hidden = self.surface_is_hidden(surface_id);
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

pub fn effective_visibility_hidden(
    globally_hidden: bool,
    output_hidden: bool,
    fullscreen_hidden: bool,
) -> bool {
    globally_hidden || output_hidden || fullscreen_hidden
}

pub fn checked_frame_buffer_size(width: u32, height: u32) -> Option<usize> {
    const MAX_FRAME_BYTES: usize = 512 * 1024 * 1024;
    let stride = width.checked_mul(4)?;
    if stride > i32::MAX as u32 || height > i32::MAX as u32 {
        return None;
    }
    let size = (stride as usize).checked_mul(height as usize)?;
    (size <= MAX_FRAME_BYTES).then_some(size)
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
    fn manual_visibility_takes_precedence_over_fullscreen_updates() {
        assert!(effective_visibility_hidden(true, false, false));
        assert!(effective_visibility_hidden(false, true, false));
        assert!(effective_visibility_hidden(false, false, true));
        assert!(effective_visibility_hidden(true, false, true));
        assert!(!effective_visibility_hidden(false, false, false));
    }

    #[test]
    fn frame_buffers_reject_overflow_and_large_allocations() {
        assert_eq!(checked_frame_buffer_size(1920, 1080), Some(1920 * 1080 * 4));
        assert_eq!(checked_frame_buffer_size(u32::MAX, 1080), None);
        assert_eq!(checked_frame_buffer_size(16000, 16000), None);
    }

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
