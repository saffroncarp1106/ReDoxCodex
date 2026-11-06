use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot};

use crate::codex::{Approval, ProviderEvent};
use crate::domain::Mode;

const FILE_LIMIT: u64 = 128 * 1024;

pub(super) fn definitions(mode: Mode) -> Value {
    let mut tools = vec![
        definition(
            "list_files",
            "List project files and directories. Read AGENTS.md before editing.",
            &["path"],
            json!({"path":{"type":"string","description":"Project-relative directory, or ."}}),
        ),
        definition(
            "read_file",
            "Read a UTF-8 project file (up to 128 KiB).",
            &["path"],
            json!({"path":{"type":"string"}}),
        ),
    ];
    if mode != Mode::Chat {
        tools.extend([
            definition("write_file", "Create or replace a UTF-8 project file using the complete new content. Existing files are backed up. Requires approval except in Full access mode.", &["path","content"], json!({"path":{"type":"string"},"content":{"type":"string"}})),
            definition("make_directory", "Create a project directory. Requires approval except in Full access mode.", &["path"], json!({"path":{"type":"string"}})),
            definition("run_command", "Run a shell command in the project for at most 30 seconds. Requires approval except in Full access mode. Runs with the user's OS permissions, without an OS sandbox.", &["command"], json!({"command":{"type":"string"}})),
        ]);
    }
    json!(tools)
}

fn definition(name: &str, description: &str, required: &[&str], properties: Value) -> Value {
    json!({"type":"function","function":{"name":name,"description":description,"parameters":{"type":"object","required":required,"properties":properties}}})
}

fn resolve_path(
    workspace: &Path,
    path: &str,
    mode: Mode,
    allow_missing: bool,
) -> Result<PathBuf, String> {
    let root = workspace
        .canonicalize()
        .map_err(|error| format!("Project folder: {error}"))?;
    let supplied = Path::new(path);
    if supplied
        .components()
        .any(|component| component == Component::ParentDir)
    {
        return Err(
            "Use project-relative paths without .., or an absolute path with Full access.".into(),
        );
    }
    let candidate = if supplied.is_absolute() {
        supplied.to_path_buf()
    } else {
        root.join(supplied)
    };
    let mut existing = candidate.as_path();
    let mut missing = Vec::new();
    while let Err(error) = std::fs::symlink_metadata(existing) {
        if error.kind() != std::io::ErrorKind::NotFound {
            return Err(error.to_string());
        }
        if !allow_missing {
            return Err(format!("File does not exist: {}", candidate.display()));
        }
        missing.push(existing.file_name().ok_or("Invalid file path.")?.to_owned());
        existing = existing.parent().ok_or("Invalid file path.")?;
    }
    let mut resolved = existing.canonicalize().map_err(|error| error.to_string())?;
    if mode != Mode::FullAccess && !resolved.starts_with(&root) {
        return Err(
            "This path is outside the selected project. Choose Full access to allow it.".into(),
        );
    }
    for part in missing.into_iter().rev() {
        resolved.push(part);
    }
    Ok(resolved)
}

async fn approve(
    description: String,
    mode: Mode,
    sender: &mpsc::Sender<ProviderEvent>,
) -> Result<(), String> {
    if mode == Mode::FullAccess {
        return Ok(());
    }
    let (response, decision) = oneshot::channel();
    sender
        .send(ProviderEvent::Approval(Approval {
            description,
            response,
        }))
        .await
        .map_err(|_| "Chat closed.")?;
    if decision.await.unwrap_or(false) {
        Ok(())
    } else {
        Err("The user rejected this action. Do not retry it without a new user request.".into())
    }
}

pub(super) async fn execute(
    name: &str,
    arguments: &Value,
    workspace: Option<&Path>,
    mode: Mode,
    sender: &mpsc::Sender<ProviderEvent>,
) -> Result<String, String> {
    let workspace = workspace.ok_or("Choose a project folder before using project tools.")?;
    let owned;
    let arguments = if let Some(text) = arguments.as_str() {
        owned = serde_json::from_str::<Value>(text).map_err(|error| error.to_string())?;
        &owned
    } else {
        arguments
    };
    if !arguments.is_object() {
        return Err("Tool arguments must be a JSON object.".into());
    }
    let argument = |key: &str| {
        arguments[key]
            .as_str()
            .ok_or_else(|| format!("Missing tool argument: {key}"))
    };
    match name {
        "list_files" => {
            let path = resolve_path(workspace, argument("path")?, mode, false)?;
            let mut entries = std::fs::read_dir(path)
                .map_err(|error| error.to_string())?
                .take(512)
                .map(|entry| {
                    let entry = entry.map_err(|error| error.to_string())?;
                    let suffix = if entry
                        .file_type()
                        .map_err(|error| error.to_string())?
                        .is_dir()
                    {
                        "/"
                    } else {
                        ""
                    };
                    Ok(format!("{}{suffix}", entry.file_name().to_string_lossy()))
                })
                .collect::<Result<Vec<_>, String>>()?;
            entries.sort();
            Ok(entries.join("\n"))
        }
        "read_file" => {
            let path = resolve_path(workspace, argument("path")?, mode, false)?;
            let metadata = std::fs::metadata(&path).map_err(|error| error.to_string())?;
            if !metadata.is_file() {
                return Err("Select a regular text file.".into());
            }
            if metadata.len() > FILE_LIMIT {
                return Err("File exceeds the 128 KiB limit.".into());
            }
            let mut bytes = Vec::new();
            tokio::fs::File::open(path)
                .await
                .map_err(|error| error.to_string())?
                .take(FILE_LIMIT + 1)
                .read_to_end(&mut bytes)
                .await
                .map_err(|error| error.to_string())?;
            if bytes.len() as u64 > FILE_LIMIT {
                return Err("File grew beyond the 128 KiB limit.".into());
            }
            String::from_utf8(bytes).map_err(|_| "This tool reads UTF-8 text files only.".into())
        }
        "write_file" | "make_directory" => {
            if mode == Mode::Chat {
                return Err("Read-only mode does not allow changes.".into());
            }
            let supplied = argument("path")?;
            let path = resolve_path(workspace, supplied, mode, true)?;
            let content = if name == "write_file" {
                argument("content")?
            } else {
                ""
            };
            if content.len() as u64 > FILE_LIMIT {
                return Err("New content exceeds the 128 KiB limit.".into());
            }
            approve(
                format!(
                    "Ollama requests {name}\nPath: {}\n\nNew content:\n{content}",
                    path.display()
                ),
                mode,
                sender,
            )
            .await?;
            let path = resolve_path(workspace, supplied, mode, true)?;
            if name == "make_directory" {
                tokio::fs::create_dir_all(&path)
                    .await
                    .map_err(|error| error.to_string())?;
                return Ok(format!("Created {}", path.display()));
            }
            if path.is_file() {
                let backup = dirs::data_local_dir()
                    .ok_or("Application data folder unavailable.")?
                    .join("ForgeDesk/agent-backups")
                    .join(uuid::Uuid::new_v4().to_string());
                tokio::fs::create_dir_all(&backup)
                    .await
                    .map_err(|error| error.to_string())?;
                tokio::fs::copy(
                    &path,
                    backup.join(path.file_name().ok_or("Invalid filename.")?),
                )
                .await
                .map_err(|error| format!("Could not preserve original file: {error}"))?;
            }
            tokio::fs::write(&path, content)
                .await
                .map_err(|error| error.to_string())?;
            Ok(format!(
                "Wrote {} ({} bytes)",
                path.display(),
                content.len()
            ))
        }
        "run_command" => {
            if mode == Mode::Chat {
                return Err("Read-only local agents do not execute shell commands.".into());
            }
            let command = argument("command")?;
            if command.len() > 16 * 1024 {
                return Err("Command is too long.".into());
            }
            let cwd = workspace
                .canonicalize()
                .map_err(|error| error.to_string())?;
            approve(format!("Ollama requests a shell command\nFolder: {}\nCommand: {command}\n\nThis runs with your OS account permissions, without an OS sandbox. Timeout: 30 seconds.", cwd.display()), mode, sender).await?;
            run_command(command, &cwd).await
        }
        _ => Err(format!("Unknown project tool: {name}")),
    }
}

async fn run_command(command: &str, cwd: &Path) -> Result<String, String> {
    #[cfg(windows)]
    let mut process = {
        let mut process = tokio::process::Command::new("cmd.exe");
        process.args(["/D", "/S", "/C", command]);
        process.creation_flags(0x08000000);
        process
    };
    #[cfg(not(windows))]
    let mut process = {
        let mut process = tokio::process::Command::new("/bin/sh");
        process.args(["-c", command]);
        process
    };
    let mut child = process
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| error.to_string())?;
    let stdout = child.stdout.take().ok_or("Command stdout unavailable.")?;
    let stderr = child.stderr.take().ok_or("Command stderr unavailable.")?;
    let work = async {
        let (stdout, stderr, status) =
            tokio::join!(read_output(stdout), read_output(stderr), child.wait());
        Ok(format!(
            "Exit: {}\nstdout (up to 64 KiB):\n{}\nstderr (up to 64 KiB):\n{}",
            status.map_err(|error| error.to_string())?,
            stdout?,
            stderr?
        ))
    };
    tokio::time::timeout(Duration::from_secs(30), work)
        .await
        .map_err(|_| "Command timed out after 30 seconds.".to_owned())?
}

async fn read_output(reader: impl tokio::io::AsyncRead + Unpin) -> Result<String, String> {
    let mut bytes = Vec::new();
    reader
        .take(64 * 1024)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| error.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

#[cfg(test)]
#[path = "../tests/transport/ollama_tools.rs"]
mod tests;
