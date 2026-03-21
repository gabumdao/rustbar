pub mod config;
pub mod wayland;
pub mod render;

use smithay_client_toolkit::registry::RegistryState;
use smithay_client_toolkit::output::OutputState;
use smithay_client_toolkit::shell::wlr_layer::LayerShell;
use wayland_client::Connection;
use calloop::{EventLoop, LoopSignal};
use calloop_wayland_source::WaylandSource;
use smithay_client_toolkit::compositor::CompositorState;

pub enum AppEvent {
    ReloadConfig,
    CheckFullscreen,
}

fn main() {
    env_logger::init();
    
    let conn = Connection::connect_to_env().expect("Could not connect to Wayland");
    let (globals, mut event_queue) = wayland_client::globals::registry_queue_init(&conn).expect("Failed to create queue");
    let qh = event_queue.handle();

    let mut event_loop: calloop::EventLoop<wayland::State> = calloop::EventLoop::try_new().unwrap();
    let loop_handle = event_loop.handle();
    let loop_signal = event_loop.get_signal();

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
        output_names: std::collections::HashMap::new(),
    };

    event_queue.roundtrip(&mut state).expect("Roundtrip failed");

    state.shm = Some(smithay_client_toolkit::shm::Shm::bind(&globals, &qh).expect("wl_shm missing"));
    state.layer_shell = Some(smithay_client_toolkit::shell::wlr_layer::LayerShell::bind(&globals, &qh).expect("zwlr_layer_shell_v1 missing"));

    for output in state.output_state.outputs() {
        use smithay_client_toolkit::output::OutputHandler;
        state.new_output(&conn, &qh, output);
    }

    calloop_wayland_source::WaylandSource::new(conn.clone(), event_queue).insert(loop_handle.clone()).unwrap();

    let (tx, rx) = calloop::channel::channel();
    
    let tx_clone = tx.clone();
    std::thread::spawn(move || {
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
            if let Ok(event) = res {
                if event.kind.is_modify() || event.kind.is_create() {
                   let _ = tx_clone.send(AppEvent::ReloadConfig);
                }
            }
        }).expect("Failed to create file watcher");
        
        use notify::Watcher;
        let _ = watcher.watch(std::path::Path::new("rustbar.toml"), notify::RecursiveMode::NonRecursive);
        
        loop { std::thread::sleep(std::time::Duration::from_secs(3600)); }
    });

    if let (Ok(xdg), Ok(sig)) = (std::env::var("XDG_RUNTIME_DIR"), std::env::var("HYPRLAND_INSTANCE_SIGNATURE")) {
        let tx_ipc = tx.clone();
        std::thread::spawn(move || {
            let socket_path = format!("{}/hypr/{}/.socket2.sock", xdg, sig);
            loop {
                if let Ok(stream) = std::os::unix::net::UnixStream::connect(&socket_path) {
                    use std::io::{BufReader, BufRead};
                    let reader = BufReader::new(stream);
                    for line in reader.lines().flatten() {
                        if line.starts_with("fullscreen>>") || line.starts_with("workspace>>") || line.starts_with("activewindow>>") || line.starts_with("focusedmon>>") {
                            let _ = tx_ipc.send(AppEvent::CheckFullscreen);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
    }

    // Trigger initial fullscreen check
    let _ = tx.send(AppEvent::CheckFullscreen);

    event_loop.handle().insert_source(rx, |event, _, state| {
        if let calloop::channel::Event::Msg(msg) = event {
            match msg {
                AppEvent::ReloadConfig => {
                    std::thread::sleep(std::time::Duration::from_millis(50));
                    state.config = config::Config::load(std::path::Path::new("rustbar.toml"));
                    state.refresh_all();
                }
                AppEvent::CheckFullscreen => {
                    state.update_fullscreen();
                }
            }
        }
    }).unwrap();

    println!("Starting rustbar...");
    event_loop.run(None, &mut state, |_| {}).unwrap();
}
