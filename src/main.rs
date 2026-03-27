pub mod config;
pub mod render;
pub mod state;
pub mod wayland;

use wayland_client::Connection;

pub enum AppEvent {
    ReloadConfig,
    CheckFullscreen,
}

fn main() {
    env_logger::init();

    if let Err(err) = run() {
        log::error!("Startup failed: {}", err);
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|err| {
        format!(
            "could not connect to the Wayland compositor from the current environment: {}",
            err
        )
    })?;
    let (globals, mut event_queue) = wayland_client::globals::registry_queue_init(&conn)
        .map_err(|err| format!("failed to create the Wayland event queue: {}", err))?;
    let qh = event_queue.handle();

    let mut event_loop: calloop::EventLoop<state::State> =
        calloop::EventLoop::try_new().map_err(|err| format!("failed to create the event loop: {}", err))?;
    let loop_handle = event_loop.handle();
    let loop_signal = event_loop.get_signal();

    let initial_config_path = config::Config::resolve_path();
    let initial_config = match config::Config::try_load_default() {
        Ok(cfg) => {
            log::info!(
                "Loaded config from {} ({} file(s) including includes)",
                initial_config_path.display(),
                cfg.loaded_files.len()
            );
            cfg
        }
        Err(err) => {
            log::warn!(
                "Could not load config from {}. Starting with built-in defaults instead. Error: {}",
                initial_config_path.display(),
                err
            );
            config::Config::default()
        }
    };

    let mut state = state::State {
        registry_state: smithay_client_toolkit::registry::RegistryState::new(&globals),
        compositor_state: smithay_client_toolkit::compositor::CompositorState::bind(&globals, &qh)
            .map_err(|_| "wl_compositor is not available; rustbar requires a Wayland compositor with wl_compositor support".to_string())?,
        output_state: smithay_client_toolkit::output::OutputState::new(&globals, &qh),
        shm: None,
        layer_shell: None,
        config: initial_config,
        loop_signal,
        active_surfaces: Vec::new(),
        dimensions: std::collections::HashMap::new(),
        pools: std::collections::HashMap::new(),
        output_names: std::collections::HashMap::new(),
        output_ids: std::collections::HashMap::new(),
        output_scales: std::collections::HashMap::new(),
        output_transforms: std::collections::HashMap::new(),
        fullscreen_hidden: std::collections::HashSet::new(),
        watcher: None,
    };

    let (tx, rx) = calloop::channel::channel();
    let tx_clone = tx.clone();
    let conn_for_reload = conn.clone();
    let qh_for_reload = qh.clone();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() {
                let _ = tx_clone.send(AppEvent::ReloadConfig);
            }
        }
    })
    .map_err(|err| format!("failed to create the config file watcher: {}", err))?;

    use notify::Watcher;
    for file in &state.config.loaded_files {
        if let Err(err) = watcher.watch(file, notify::RecursiveMode::NonRecursive) {
            log::warn!(
                "Failed to watch config file {} for live reload: {}",
                file.display(),
                err
            );
        }
    }
    state.watcher = Some(watcher);

    event_queue
        .roundtrip(&mut state)
        .map_err(|err| format!("initial Wayland roundtrip failed: {}", err))?;

    state.shm = Some(
        smithay_client_toolkit::shm::Shm::bind(&globals, &qh)
            .map_err(|_| "wl_shm is not available; rustbar requires shared-memory buffer support".to_string())?,
    );
    state.layer_shell = Some(
        smithay_client_toolkit::shell::wlr_layer::LayerShell::bind(&globals, &qh)
            .map_err(|_| "zwlr_layer_shell_v1 is not available; rustbar requires the wlr-layer-shell protocol".to_string())?,
    );

    for output in state.output_state.outputs() {
        use smithay_client_toolkit::output::OutputHandler;
        state.new_output(&conn, &qh, output);
    }

    calloop_wayland_source::WaylandSource::new(conn.clone(), event_queue)
        .insert(loop_handle.clone())
        .map_err(|err| format!("failed to insert the Wayland source into the event loop: {}", err))?;

    if let (Ok(xdg), Ok(sig)) = (
        std::env::var("XDG_RUNTIME_DIR"),
        std::env::var("HYPRLAND_INSTANCE_SIGNATURE"),
    ) {
        let tx_ipc = tx.clone();
        std::thread::spawn(move || {
            let socket_path = format!("{}/hypr/{}/.socket2.sock", xdg, sig);
            loop {
                if let Ok(stream) = std::os::unix::net::UnixStream::connect(&socket_path) {
                    use std::io::{BufRead, BufReader};
                    let reader = BufReader::new(stream);
                    for line in reader.lines().flatten() {
                        if line.starts_with("fullscreen>>")
                            || line.starts_with("workspace>>")
                            || line.starts_with("activewindow>>")
                            || line.starts_with("focusedmon>>")
                        {
                            let _ = tx_ipc.send(AppEvent::CheckFullscreen);
                        }
                    }
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
    }

    let _ = tx.send(AppEvent::CheckFullscreen);

    event_loop
        .handle()
        .insert_source(rx, move |event, _, state| {
            if let calloop::channel::Event::Msg(msg) = event {
                match msg {
                    AppEvent::ReloadConfig => {
                        std::thread::sleep(std::time::Duration::from_millis(50));
                        let resolved_path = config::Config::resolve_path();
                        match config::Config::try_load_default() {
                            Ok(new_config) => {
                                let old_files = state.config.loaded_files.clone();
                                log::info!(
                                    "Reloaded config from {} ({} file(s) including includes)",
                                    resolved_path.display(),
                                    new_config.loaded_files.len()
                                );
                                state.config = new_config;

                                if let Some(watcher) = &mut state.watcher {
                                    use notify::Watcher;
                                    for f in old_files {
                                        if let Err(err) = watcher.unwatch(&f) {
                                            log::warn!(
                                                "Failed to stop watching old config file {}: {}",
                                                f.display(),
                                                err
                                            );
                                        }
                                    }
                                    for f in &state.config.loaded_files {
                                        if let Err(err) =
                                            watcher.watch(f, notify::RecursiveMode::NonRecursive)
                                        {
                                            log::warn!(
                                                "Failed to watch config file {} after reload: {}",
                                                f.display(),
                                                err
                                            );
                                        }
                                    }
                                }

                                state.sync_outputs(&conn_for_reload, &qh_for_reload);
                                state.refresh_all();
                            }
                            Err(err) => {
                                log::warn!(
                                    "Skipping config reload for {} and keeping the previous config. Error: {}",
                                    resolved_path.display(),
                                    err
                                );
                            }
                        }
                    }
                    AppEvent::CheckFullscreen => {
                        state.update_fullscreen(&qh_for_reload);
                    }
                }
            }
        })
        .map_err(|err| format!("failed to insert the app event source into the event loop: {}", err))?;

    println!("Starting rustbar...");
    event_loop
        .run(None, &mut state, |_| {})
        .map_err(|err| format!("event loop terminated unexpectedly: {}", err))?;
    Ok(())
}
