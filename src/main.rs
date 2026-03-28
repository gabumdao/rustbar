pub mod config;
pub mod render;
pub mod state;
pub mod wayland;

use std::{io::{BufRead, BufReader, Write}, os::unix::net::UnixListener, path::PathBuf};
use wayland_client::Connection;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    Reload,
    Refresh,
    CheckFullscreen,
    Show,
    Hide,
    Toggle,
    ShowOutput(String),
    HideOutput(String),
    ToggleOutput(String),
    Status,
    Quit,
}

pub enum AppEvent {
    ReloadConfig(std::time::Instant),
    CheckFullscreen,
    Command(IpcCommand, Option<std::sync::mpsc::Sender<String>>),
}

fn parse_ipc_command(input: &str) -> Option<IpcCommand> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut parts = trimmed.split_whitespace();
    let command = parts.next()?.to_ascii_lowercase();
    let remainder = parts.collect::<Vec<_>>().join(" ");
    let output = if remainder.is_empty() {
        None
    } else {
        Some(remainder)
    };

    match command.as_str() {
        "reload" => Some(IpcCommand::Reload),
        "refresh" => Some(IpcCommand::Refresh),
        "fullscreen" | "check-fullscreen" | "check_fullscreen" => Some(IpcCommand::CheckFullscreen),
        "show" => output.map(IpcCommand::ShowOutput).or(Some(IpcCommand::Show)),
        "hide" => output.map(IpcCommand::HideOutput).or(Some(IpcCommand::Hide)),
        "toggle" => output.map(IpcCommand::ToggleOutput).or(Some(IpcCommand::Toggle)),
        "status" | "info" => Some(IpcCommand::Status),
        "quit" | "exit" | "stop" => Some(IpcCommand::Quit),
        _ => None,
    }
}

fn control_socket_path() -> Option<PathBuf> {
    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(PathBuf::from)
        .map(|dir| dir.join("rustbar.sock"))
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
        render_caches: std::collections::HashMap::new(),
        last_reload_at: None,
        fullscreen_hidden: std::collections::HashSet::new(),
        manually_hidden: false,
        manually_hidden_outputs: std::collections::HashSet::new(),
        watcher: None,
    };

    let (tx, rx) = calloop::channel::channel();
    let tx_clone = tx.clone();
    let conn_for_reload = conn.clone();
    let qh_for_reload = qh.clone();

    let mut watcher = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(event) = res {
            if event.kind.is_modify() || event.kind.is_create() {
                let _ = tx_clone.send(AppEvent::ReloadConfig(std::time::Instant::now()));
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


    if let Some(socket_path) = control_socket_path() {
        let tx_control = tx.clone();
        match std::fs::remove_file(&socket_path) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => log::warn!(
                "Failed to remove stale control socket {}: {}",
                socket_path.display(),
                err
            ),
        }

        match UnixListener::bind(&socket_path) {
            Ok(listener) => {
                log::info!(
                    "Listening for control commands on {}",
                    socket_path.display()
                );
                std::thread::spawn(move || {
                    for stream in listener.incoming() {
                        let mut stream = match stream {
                            Ok(stream) => stream,
                            Err(err) => {
                                log::warn!("Control socket accept failed: {}", err);
                                continue;
                            }
                        };

                        let mut line = String::new();
                        let read_result = {
                            let mut reader = BufReader::new(&stream);
                            reader.read_line(&mut line)
                        };
                        if let Err(err) = read_result {
                            let _ = stream.write_all(
                                format!("error: failed to read command: {}\n", err).as_bytes(),
                            );
                            continue;
                        }

                        let trimmed = line.trim();
                        match parse_ipc_command(trimmed) {
                            Some(command) => {
                                let wants_response = matches!(command, IpcCommand::Status);
                                let response = if wants_response {
                                    let (tx_response, rx_response) = std::sync::mpsc::channel();
                                    match tx_control.send(AppEvent::Command(command, Some(tx_response))) {
                                        Ok(()) => match rx_response.recv_timeout(std::time::Duration::from_secs(1)) {
                                            Ok(response) => response,
                                            Err(_) => "error: timed out waiting for rustbar status\n".to_string(),
                                        },
                                        Err(_) => "error: rustbar is shutting down\n".to_string(),
                                    }
                                } else if tx_control.send(AppEvent::Command(command, None)).is_ok() {
                                    "ok\n".to_string()
                                } else {
                                    "error: rustbar is shutting down\n".to_string()
                                };
                                let _ = stream.write_all(response.as_bytes());
                            }
                            None => {
                                let _ = stream.write_all(
                                    b"error: unknown command. use reload, refresh, fullscreen, show, hide, toggle, status, or quit\n",
                                );
                            }
                        }
                    }
                });
            }
            Err(err) => {
                log::warn!(
                    "Failed to bind control socket {}: {}",
                    socket_path.display(),
                    err
                );
            }
        }
    }

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
                    AppEvent::ReloadConfig(triggered_at) => {
                        let debounce_window = std::time::Duration::from_millis(200);
                        if state
                            .last_reload_at
                            .map(|last| triggered_at.saturating_duration_since(last) < debounce_window)
                            .unwrap_or(false)
                        {
                            return;
                        }
                        state.last_reload_at = Some(triggered_at);
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
                                    let old_set: std::collections::BTreeSet<_> =
                                        old_files.iter().cloned().collect();
                                    let new_set: std::collections::BTreeSet<_> =
                                        state.config.loaded_files.iter().cloned().collect();

                                    for f in old_set.difference(&new_set) {
                                        if let Err(err) = watcher.unwatch(f) {
                                            log::warn!(
                                                "Failed to stop watching old config file {}: {}",
                                                f.display(),
                                                err
                                            );
                                        }
                                    }
                                    for f in new_set.difference(&old_set) {
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
                    AppEvent::Command(command, response_tx) => match command {
                        IpcCommand::Reload => {
                            let now = std::time::Instant::now();
                            let debounce_window = std::time::Duration::from_millis(200);
                            if state
                                .last_reload_at
                                .map(|last| now.saturating_duration_since(last) < debounce_window)
                                .unwrap_or(false)
                            {
                                return;
                            }
                            state.last_reload_at = Some(now);
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
                                        let old_set: std::collections::BTreeSet<_> =
                                            old_files.iter().cloned().collect();
                                        let new_set: std::collections::BTreeSet<_> =
                                            state.config.loaded_files.iter().cloned().collect();

                                        for f in old_set.difference(&new_set) {
                                            if let Err(err) = watcher.unwatch(f) {
                                                log::warn!(
                                                    "Failed to stop watching old config file {}: {}",
                                                    f.display(),
                                                    err
                                                );
                                            }
                                        }
                                        for f in new_set.difference(&old_set) {
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
                        IpcCommand::Refresh => {
                            state.refresh_all();
                        }
                        IpcCommand::CheckFullscreen => {
                            state.update_fullscreen(&qh_for_reload);
                        }
                        IpcCommand::Show => {
                            state.set_manual_hidden(false, &qh_for_reload);
                        }
                        IpcCommand::Hide => {
                            state.set_manual_hidden(true, &qh_for_reload);
                        }
                        IpcCommand::Toggle => {
                            let hidden = !state.manually_hidden;
                            state.set_manual_hidden(hidden, &qh_for_reload);
                        }
                        IpcCommand::ShowOutput(output) => {
                            state.set_output_hidden(&output, false, &qh_for_reload);
                        }
                        IpcCommand::HideOutput(output) => {
                            state.set_output_hidden(&output, true, &qh_for_reload);
                        }
                        IpcCommand::ToggleOutput(output) => {
                            state.toggle_output_hidden(&output, &qh_for_reload);
                        }
                        IpcCommand::Status => {
                            if let Some(response_tx) = response_tx {
                                let _ = response_tx.send(state.status_summary());
                            }
                        }
                        IpcCommand::Quit => {
                            state.loop_signal.stop();
                        }
                    },
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


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ipc_command_recognizes_supported_commands() {
        assert_eq!(parse_ipc_command("reload"), Some(IpcCommand::Reload));
        assert_eq!(parse_ipc_command("refresh"), Some(IpcCommand::Refresh));
        assert_eq!(parse_ipc_command("fullscreen"), Some(IpcCommand::CheckFullscreen));
        assert_eq!(parse_ipc_command("show"), Some(IpcCommand::Show));
        assert_eq!(parse_ipc_command("hide"), Some(IpcCommand::Hide));
        assert_eq!(parse_ipc_command("toggle"), Some(IpcCommand::Toggle));
        assert_eq!(parse_ipc_command("show DP-1"), Some(IpcCommand::ShowOutput("DP-1".to_string())));
        assert_eq!(parse_ipc_command("hide HDMI-A-1"), Some(IpcCommand::HideOutput("HDMI-A-1".to_string())));
        assert_eq!(parse_ipc_command("toggle eDP-1"), Some(IpcCommand::ToggleOutput("eDP-1".to_string())));
        assert_eq!(parse_ipc_command("status"), Some(IpcCommand::Status));
        assert_eq!(parse_ipc_command("quit"), Some(IpcCommand::Quit));
    }

    #[test]
    fn parse_ipc_command_accepts_aliases_and_whitespace() {
        assert_eq!(parse_ipc_command("  check_fullscreen  "), Some(IpcCommand::CheckFullscreen));
        assert_eq!(parse_ipc_command("info"), Some(IpcCommand::Status));
        assert_eq!(parse_ipc_command("  show   DP-1  "), Some(IpcCommand::ShowOutput("DP-1".to_string())));
        assert_eq!(parse_ipc_command("EXIT"), Some(IpcCommand::Quit));
        assert_eq!(parse_ipc_command("stop"), Some(IpcCommand::Quit));
    }

    #[test]
    fn parse_ipc_command_rejects_unknown_commands() {
        assert_eq!(parse_ipc_command("bogus"), None);
        assert_eq!(parse_ipc_command(""), None);
    }
}
