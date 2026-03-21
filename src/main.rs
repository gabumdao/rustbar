pub mod config;
pub mod wayland;
pub mod render;

use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use wayland_client::Connection;
use calloop::{EventLoop, LoopSignal};
use calloop_wayland_source::WaylandSource;

use smithay_client_toolkit::compositor::CompositorState;

fn main() {
    env_logger::init();
    
    let conn = Connection::connect_to_env().expect("Could not connect to Wayland");
    let (globals, mut event_queue) = wayland_client::globals::registry_queue_init(&conn).expect("Failed to create queue");
    let qh = event_queue.handle();

    let mut event_loop: calloop::EventLoop<wayland::State> = calloop::EventLoop::try_new().unwrap();
    let loop_handle = event_loop.handle();
    let loop_signal = event_loop.get_signal();

    calloop_wayland_source::WaylandSource::new(conn.clone(), event_queue).insert(loop_handle.clone()).unwrap();

    let mut state = wayland::State {
        registry_state: smithay_client_toolkit::registry::RegistryState::new(&globals),
        compositor_state: smithay_client_toolkit::compositor::CompositorState::bind(&globals, &qh).expect("wl_compositor missing"),
        output_state: smithay_client_toolkit::output::OutputState::new(&globals, &qh),
        shm: None,
        layer_shell: None,
        config: config::Config::load(std::path::Path::new("rustbar.toml")),
        loop_signal,
        active_surfaces: Vec::new(),
        dimensions: std::collections::HashMap::new(),
        pools: std::collections::HashMap::new(),
    };

    let (tx, rx) = calloop::channel::channel();
    
    std::thread::spawn(move || {
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() {
                   let _ = tx.send(());
                }
            }
        }).expect("Failed to create file watcher");
        
        use notify::Watcher;
        watcher.watch(std::path::Path::new("rustbar.toml"), notify::RecursiveMode::NonRecursive)
               .expect("Failed to watch rustbar.toml");
        
        loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
    });

    event_loop.handle().insert_source(rx, |event, _, state| {
        if let calloop::channel::Event::Msg(_) = event {
            std::thread::sleep(std::time::Duration::from_millis(50));
            state.config = config::Config::load(std::path::Path::new("rustbar.toml"));
            state.refresh_all();
        }
    }).unwrap();

    println!("Starting rustbar...");
    event_loop.run(None, &mut state, |_| {}).unwrap();
}
