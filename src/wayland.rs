use crate::state::{State, effective_namespace, map_layer};
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
    shm::{Shm, ShmHandler},
};
use wayland_client::{
    Connection, Proxy, QueueHandle, delegate_noop,
    protocol::{wl_output, wl_region::WlRegion, wl_surface::WlSurface},
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
        surface: &WlSurface,
        _new_factor: i32,
    ) {
        if let Some(surface_id) = self.surface_id_for_wl_surface(surface) {
            self.output_scales
                .insert(surface_id, crate::state::normalized_scale(_new_factor));
            self.pools.remove(&surface_id);
            self.refresh_surface(surface_id);
        }
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &WlSurface,
        _new_transform: wl_output::Transform,
    ) {
        if let Some(surface_id) = self.surface_id_for_wl_surface(surface) {
            self.output_transforms.insert(surface_id, _new_transform);
            self.pools.remove(&surface_id);
            self.refresh_surface(surface_id);
        }
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
        if self
            .surface_id_for_output_id(output.id().protocol_id())
            .is_some()
        {
            return;
        }

        if let Some(info) = self.output_state.info(&output) {
            let name = info.name.unwrap_or_else(|| "Unknown".to_string());
            let scale = crate::state::normalized_scale(info.scale_factor);
            let transform = info.transform;
            let m_config = self.config.for_monitor(&name);
            if !m_config.enabled.unwrap_or(true) {
                return;
            }

            if let Some(layer_shell) = &self.layer_shell {
                let surface = self.compositor_state.create_surface(qh);
                let layer_config = m_config.layer.unwrap_or_default();

                let layer = layer_shell.create_layer_surface(
                    qh,
                    surface,
                    map_layer(&layer_config.level),
                    Some(effective_namespace(&layer_config.namespace)),
                    Some(&output),
                );

                layer.set_size(0, 0);
                layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
                let (scale, transform) = self.apply_surface_output_state(&layer, scale, transform);
                self.apply_layer_surface_config(&layer, qh, &layer_config, false);

                let id = layer.wl_surface().id().protocol_id();
                self.output_names.insert(id, name);
                self.output_ids.insert(id, output.id().protocol_id());
                self.output_scales.insert(id, scale);
                self.output_transforms.insert(id, transform);
                self.active_surfaces.push(layer);
            }
        }
    }

    fn update_output(
        &mut self,
        conn: &Connection,
        qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.reconcile_output(conn, qh, output);
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
        self.refresh_surface(id_surf);
    }
}

delegate_registry!(State);
delegate_compositor!(State);
delegate_output!(State);
delegate_shm!(State);
delegate_layer!(State);
delegate_noop!(State: ignore WlRegion);
