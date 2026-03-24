use crate::state::{State, map_keyboard_interactivity, map_layer};
use smithay_client_toolkit::{
    compositor::CompositorHandler,
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    shell::{
        WaylandSurface,
        wlr_layer::{Anchor, LayerShellHandler, LayerSurface, LayerSurfaceConfigure},
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, Proxy, QueueHandle, delegate_noop,
    protocol::{wl_output, wl_region::WlRegion, wl_shm, wl_surface::WlSurface},
};

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_factor: i32,
    ) {
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _time: u32,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        if let Some(info) = self.output_state.info(&output) {
            let name = info.name.unwrap_or_else(|| "Unknown".to_string());
            let m_config = self.config.for_monitor(&name);
            if !m_config.enabled.unwrap_or(true) {
                return;
            }

            if let Some(layer_shell) = &self.layer_shell {
                let surface = self.compositor_state.create_surface(qh);
                let layer_config = m_config.layer.unwrap_or_default();

                if layer_config.click_through {
                    let region = self.compositor_state.wl_compositor().create_region(qh, ());
                    surface.set_input_region(Some(&region));
                    region.destroy();
                } else {
                    surface.set_input_region(None);
                }

                let layer = layer_shell.create_layer_surface(
                    qh,
                    surface,
                    map_layer(&layer_config.level),
                    Some(layer_config.namespace.clone()),
                    Some(&output),
                );

                layer.set_size(0, 0);
                layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
                layer.set_exclusive_zone(layer_config.exclusive_zone);
                layer.set_keyboard_interactivity(map_keyboard_interactivity(
                    &layer_config.keyboard_interactivity,
                ));
                layer.commit();

                let id = layer.wl_surface().id().protocol_id();
                self.output_names.insert(id, name);
                self.output_ids.insert(id, output.id().protocol_id());
                self.active_surfaces.push(layer);
            }
        }
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _output: wl_output::WlOutput,
    ) {
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        let output_id = output.id().protocol_id();
        let surface_ids: Vec<u32> = self
            .output_ids
            .iter()
            .filter_map(|(surface_id, stored_output_id)| {
                if *stored_output_id == output_id {
                    Some(*surface_id)
                } else {
                    None
                }
            })
            .collect();

        for surface_id in surface_ids {
            self.remove_surface(surface_id);
        }
    }
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        self.shm.as_mut().unwrap()
    }
}

impl LayerShellHandler for State {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        let id = layer.wl_surface().id().protocol_id();
        self.active_surfaces
            .retain(|s| s.wl_surface() != layer.wl_surface());
        self.forget_surface(id);
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        let (width, height) = configure.new_size;
        if width == 0 || height == 0 {
            return;
        }

        let id_surf = layer.wl_surface().id().protocol_id();
        self.dimensions.insert(id_surf, (width, height));

        let monitor_name = self
            .output_names
            .get(&id_surf)
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        let m_config = self.config.for_monitor(&monitor_name);
        let pixels = if m_config.enabled.unwrap_or(true) {
            crate::render::draw_frame(width, height, &m_config)
        } else {
            vec![0; (width * height * 4) as usize]
        };

        if let Some(shm) = self.shm.as_mut() {
            let size = (width * height * 4) as usize;
            let (pool, active_buf) = self.pools.entry(id_surf).or_insert_with(|| {
                (
                    SlotPool::new(size, shm).expect("Failed to create slot pool"),
                    None,
                )
            });

            if pool.resize(size).is_err() {
                return;
            }
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

delegate_registry!(State);
delegate_compositor!(State);
delegate_output!(State);
delegate_shm!(State);
delegate_layer!(State);
delegate_noop!(State: ignore WlRegion);
