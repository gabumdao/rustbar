use serde_json::Value;
use std::{
    env,
    io::{Read, Write},
    os::unix::net::UnixStream,
    path::PathBuf,
    process,
};

fn usage() -> &'static str {
    "Usage: rustbarctl [--socket PATH] <command> [args...]

Global options:
  -s, --socket PATH  Override the control socket path

Commands:
  reload           Reload the active config; add --json for machine-readable state
  refresh          Redraw all active surfaces; add --json for machine-readable state
  fullscreen       Rerun the fullscreen-state check; add --json for machine-readable state
  show             Force all rustbar surfaces visible, or one output if given; add --json for machine-readable state
  hide             Force all rustbar surfaces hidden, or one output if given; add --json for machine-readable state
  toggle           Toggle manual visibility for all surfaces, or one output if given; add --json for machine-readable state
  status           Print a live rustbar status summary, or one output if given; add --json for machine-readable output
  list-outputs     Print all active outputs with runtime details; add --json for machine-readable output
  complete-outputs Print active output names one per line, optionally filtered by a prefix
  quit             Stop rustbar cleanly
  help             Show this help
"
}

fn command_is_supported(command: &str) -> bool {
    matches!(
        command,
        "reload"
            | "refresh"
            | "fullscreen"
            | "check-fullscreen"
            | "check_fullscreen"
            | "show"
            | "hide"
            | "toggle"
            | "status"
            | "info"
            | "list-outputs"
            | "list_outputs"
            | "outputs"
            | "quit"
            | "exit"
            | "stop"
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Action {
    Help,
    Request(String),
    CompleteOutputs(Option<String>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedCli {
    socket_override: Option<PathBuf>,
    action: Action,
}

fn default_socket_path() -> Result<PathBuf, String> {
    let runtime_dir = env::var("XDG_RUNTIME_DIR")
        .map_err(|_| "XDG_RUNTIME_DIR is not set.".to_string())?;
    Ok(PathBuf::from(runtime_dir).join("rustbar.sock"))
}

fn parse_cli(args: &[String]) -> Result<ParsedCli, String> {
    let mut socket_override = None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "-s" | "--socket" => {
                let Some(path) = args.get(index + 1) else {
                    return Err("Missing value for --socket.

".to_string() + usage());
                };
                socket_override = Some(PathBuf::from(path));
                index += 2;
            }
            "-h" | "--help" | "help" => {
                return Ok(ParsedCli {
                    socket_override,
                    action: Action::Help,
                });
            }
            _ => break,
        }
    }

    if index >= args.len() {
        return Ok(ParsedCli {
            socket_override,
            action: Action::Help,
        });
    }

    let command = &args[index];
    let remainder = &args[index + 1..];

    if command == "complete-outputs" {
        let prefix = match remainder {
            [] => None,
            [single] => Some(single.clone()),
            _ => {
                return Err(
                    "complete-outputs accepts at most one optional prefix argument.

"
                        .to_string()
                        + usage(),
                )
            }
        };
        return Ok(ParsedCli {
            socket_override,
            action: Action::CompleteOutputs(prefix),
        });
    }

    if !command_is_supported(command) {
        return Err(format!("Unknown command: {}

{}", command, usage()));
    }

    let request = args[index..].join(" ");
    Ok(ParsedCli {
        socket_override,
        action: Action::Request(request),
    })
}

fn socket_path(override_path: Option<PathBuf>) -> Result<PathBuf, String> {
    match override_path {
        Some(path) => Ok(path),
        None => default_socket_path(),
    }
}

fn send_request(socket_path: &PathBuf, request: &str) -> Result<String, String> {
    let mut stream = UnixStream::connect(socket_path).map_err(|err| {
        format!(
            "Rustbar control socket not found or unavailable at {}: {}",
            socket_path.display(),
            err
        )
    })?;
    stream
        .write_all(format!("{}
", request).as_bytes())
        .map_err(|err| format!("Failed to send request to {}: {}", socket_path.display(), err))?;

    let mut response = String::new();
    stream
        .read_to_string(&mut response)
        .map_err(|err| format!("Failed to read response from {}: {}", socket_path.display(), err))?;

    if response.is_empty() {
        return Err("Rustbar returned an empty response.".to_string());
    }

    Ok(response)
}

fn extract_output_names_from_json(response: &str) -> Result<Vec<String>, String> {
    let value: Value = serde_json::from_str(response)
        .map_err(|err| format!("Failed to parse rustbar response as JSON: {}", err))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("error");
    if status != "ok" {
        return Err(format!("Rustbar returned a non-ok status: {}", status));
    }

    let outputs = value
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "Rustbar JSON response did not include an outputs array.".to_string())?;

    let mut names = Vec::new();
    for output in outputs {
        if let Some(name) = output.get("output").and_then(Value::as_str) {
            names.push(name.to_string());
        }
    }
    names.sort();
    names.dedup();
    Ok(names)
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let parsed = parse_cli(&args)?;

    match parsed.action {
        Action::Help => {
            print!("{}", usage());
            Ok(0)
        }
        Action::Request(request) => {
            let socket_path = socket_path(parsed.socket_override)?;
            let response = send_request(&socket_path, &request)?;
            print!("{}", response);
            Ok(if response.starts_with("error:") { 1 } else { 0 })
        }
        Action::CompleteOutputs(prefix) => {
            let socket_path = socket_path(parsed.socket_override)?;
            let response = send_request(&socket_path, "list-outputs --json")?;
            if response.starts_with("error:") {
                print!("{}", response);
                return Ok(1);
            }
            let names = extract_output_names_from_json(&response)?;
            for name in names {
                if prefix.as_ref().map(|p| name.starts_with(p)).unwrap_or(true) {
                    println!("{}", name);
                }
            }
            Ok(0)
        }
    }
}

fn main() {
    match run() {
        Ok(code) => process::exit(code),
        Err(message) => {
            eprintln!("{}", message);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cli_accepts_known_request_commands() {
        let parsed = parse_cli(&[
            "--socket".to_string(),
            "/tmp/rustbar.sock".to_string(),
            "status".to_string(),
            "DP-1".to_string(),
            "--json".to_string(),
        ])
        .unwrap();
        assert_eq!(
            parsed,
            ParsedCli {
                socket_override: Some(PathBuf::from("/tmp/rustbar.sock")),
                action: Action::Request("status DP-1 --json".to_string()),
            }
        );
    }

    #[test]
    fn parse_cli_accepts_complete_outputs() {
        let parsed = parse_cli(&["complete-outputs".to_string(), "DP".to_string()]).unwrap();
        assert_eq!(
            parsed,
            ParsedCli {
                socket_override: None,
                action: Action::CompleteOutputs(Some("DP".to_string())),
            }
        );
    }

    #[test]
    fn parse_cli_returns_help_when_no_command_is_given() {
        let parsed = parse_cli(&[]).unwrap();
        assert_eq!(parsed.action, Action::Help);
    }

    #[test]
    fn parse_cli_rejects_unknown_commands() {
        assert!(parse_cli(&["bogus".to_string()]).is_err());
    }

    #[test]
    fn extract_output_names_from_json_reads_output_names() {
        let names = extract_output_names_from_json(
            r#"{
  "status": "ok",
  "outputs": [
    { "output": "HDMI-A-1" },
    { "output": "DP-1" },
    { "output": "DP-1" }
  ]
}"#,
        )
        .unwrap();
        assert_eq!(names, vec!["DP-1".to_string(), "HDMI-A-1".to_string()]);
    }
}
