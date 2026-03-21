use smithay_client_toolkit::{
    delegate_output, delegate_registry, delegate_shm, delegate_layer, delegate_compositor,
    output::{OutputHandler, OutputState},
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    compositor::{CompositorHandler, CompositorState},
    shm::{ShmHandler, Shm, slot::{SlotPool, Buffer}},
    shell::{
        wlr_layer::{LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure, Layer, Anchor, KeyboardInteractivity},
        WaylandSurface,
    },
};
use wayland_client::{
    protocol::{wl_output, wl_registry, wl_shm, wl_surface::WlSurface, wl_region::WlRegion},
    Connection, QueueHandle, Proxy, delegate_noop,
};
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
}

impl State {
    pub fn refresh_all(&mut self) {
        for layer in &self.active_surfaces {
            let id = layer.wl_surface().id().protocol_id();
            if let Some(&(width, height)) = self.dimensions.get(&id) {
                let m_config = self.config.for_monitor("default");
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
}

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

impl CompositorHandler for State {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_factor: i32) {}
    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _new_transform: wl_output::Transform) {}
    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _time: u32) {}
    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &wl_output::WlOutput) {}
    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &WlSurface, _output: &wl_output::WlOutput) {}
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _conn: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        if let Some(info) = self.output_state.info(&output) {
            let name = info.name.unwrap_or_else(|| "Unknown".to_string());
            let m_config = self.config.for_monitor(&name);
            if !m_config.enabled.unwrap_or(true) { return; }

            if let Some(layer_shell) = &self.layer_shell {
                let surface = self.compositor_state.create_surface(qh);
                
                // Create an empty input region so clicks pass through
                let region = self.compositor_state.wl_compositor().create_region(qh, ());
                surface.set_input_region(Some(&region));
                region.destroy();

                let layer = layer_shell.create_layer_surface(
                    qh,
                    surface,
                    Layer::Overlay,
                    Some(name.clone()),
                    Some(&output),
                );
                
                layer.set_size(0, 0);
                layer.set_anchor(Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT);
                layer.set_exclusive_zone(0);
                layer.set_keyboard_interactivity(KeyboardInteractivity::None);
                layer.commit();
                
                self.active_surfaces.push(layer);
            }
        }
    }
    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        self.shm.as_mut().unwrap()
    }
}

impl LayerShellHandler for State {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface) {
        self.active_surfaces.retain(|s| s.wl_surface() != layer.wl_surface());
        let id = layer.wl_surface().id().protocol_id();
        self.pools.remove(&id);
    }
    
    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, layer: &LayerSurface, configure: LayerSurfaceConfigure, _serial: u32) {
        let (width, height) = configure.new_size;
        if width == 0 || height == 0 { return; }
        
        let id_surf = layer.wl_surface().id().protocol_id();
        self.dimensions.insert(id_surf, (width, height));
        
        // Find monitor name from output to apply config? Defaulting to "default" for now
        let m_config = self.config.for_monitor("default");
        let pixels = crate::render::draw_frame(width, height, &m_config);
        let id_surf = layer.wl_surface().id().protocol_id();
        
        if let Some(shm) = self.shm.as_mut() {
            let size = (width * height * 4) as usize;
            
            let (pool, active_buf) = self.pools.entry(id_surf).or_insert_with(|| {
                (SlotPool::new(size, shm).expect("Failed to create slot pool"), None)
            });
            
            if let Err(_) = pool.resize(size) {
                return;
            }
            if let Ok((buffer, canvas)) = pool.create_buffer(
                width as i32, 
                height as i32, 
                width as i32 * 4, 
                wl_shm::Format::Argb8888
            ) {
                canvas.copy_from_slice(&pixels);
                
                layer.wl_surface().attach(Some(buffer.wl_buffer()), 0, 0);
                // Inform server we changed the buffer
                layer.wl_surface().damage_buffer(0, 0, width as i32, height as i32);
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
