use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[derive(serde::Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
    #[serde(default)]
    pub assets: Vec<Asset>,
}

#[derive(serde::Deserialize)]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: u64,
}

#[derive(Default)]
pub struct Updates {
    pub status: String,
    pub startup_checked: bool,
    pub show_prompt: bool,
    installing: Option<Receiver<Result<std::path::PathBuf, String>>>,
    pending: Option<Receiver<Option<Release>>>,
    pub available: Option<Release>,
}

impl Updates {
    pub fn checking(&self) -> bool {
        self.pending.is_some() || self.installing.is_some()
    }

    pub fn check(&mut self, repaint: impl FnOnce() + Send + 'static) {
        if self.pending.is_none() {
            self.available = None;
            self.status = "Checking for updates?".into();
            let repository = std::env::var("REDOX_UPDATE_REPOSITORY")
                .unwrap_or_else(|_| "saffroncarp1106/ReDoxCodex".into());
            let repository = repository
                .trim_end_matches('/')
                .trim_start_matches("https://github.com/");
            if repository.split('/').count() != 2
                || !repository
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || "-_/ .".replace(' ', "").contains(c))
            {
                self.status = "Invalid update repository.".into();
                return;
            }
            let endpoint = format!("https://api.github.com/repos/{repository}/releases/latest");
            let (sender, receiver) = mpsc::channel();
            self.pending = Some(receiver);
            std::thread::spawn(move || {
                let release = tokio::runtime::Runtime::new().ok().and_then(|runtime| {
                    runtime.block_on(async {
                        let client = reqwest::Client::builder()
                            .timeout(Duration::from_secs(10))
                            .user_agent("ReDox-update-check")
                            .build()
                            .ok()?;
                        let mut request = client.get(endpoint);
                        if let Ok(token) = std::env::var("REDOX_UPDATE_TOKEN") {
                            request = request.bearer_auth(token);
                        }
                        request
                            .send()
                            .await
                            .ok()?
                            .error_for_status()
                            .ok()?
                            .json::<Release>()
                            .await
                            .ok()
                    })
                });
                let _ = sender.send(release);
                repaint();
            });
        }
    }

    pub fn install(&mut self, repaint: impl FnOnce() + Send + 'static) {
        if self.checking() {
            return;
        }
        let Some(asset) = self
            .available
            .as_ref()
            .and_then(|r| r.assets.iter().find(|a| a.name == "ReDox-Windows-x64.exe"))
        else {
            self.status = "This release has no Windows update executable.".into();
            return;
        };
        if !cfg!(windows) {
            self.status = "In-place updating currently supports Windows only.".into();
            return;
        }
        let url = asset.url.clone();
        let size = asset.size;
        if !url.starts_with("https://api.github.com/repos/") || size == 0 || size > 200_000_000 {
            self.status = "Invalid update asset.".into();
            return;
        }
        let (sender, receiver) = mpsc::channel();
        self.installing = Some(receiver);
        self.status = "Downloading update. ReDox will restart when ready?".into();
        std::thread::spawn(move || {
            let result = (|| -> Result<std::path::PathBuf, String> {
                let runtime = tokio::runtime::Runtime::new().map_err(|e| e.to_string())?;
                runtime.block_on(async {
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(180))
                        .user_agent("ReDox-updater")
                        .build()
                        .map_err(|e| e.to_string())?;
                    let mut request = client.get(url).header("Accept", "application/octet-stream");
                    if let Ok(token) = std::env::var("REDOX_UPDATE_TOKEN") {
                        request = request.bearer_auth(token);
                    }
                    let bytes = request
                        .send()
                        .await
                        .map_err(|e| e.to_string())?
                        .error_for_status()
                        .map_err(|e| e.to_string())?
                        .bytes()
                        .await
                        .map_err(|e| e.to_string())?;
                    if bytes.len() as u64 != size || !bytes.starts_with(b"MZ") {
                        return Err(
                            "Downloaded update is not the expected Windows executable.".into()
                        );
                    }
                    let directory = std::env::current_exe()
                        .map_err(|e| e.to_string())?
                        .parent()
                        .ok_or("Missing installation folder")?
                        .join("temp")
                        .join(format!("update-{}", uuid::Uuid::new_v4()));
                    std::fs::create_dir_all(&directory).map_err(|e| e.to_string())?;
                    let path = directory.join("redox-new.exe");
                    std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
                    Ok(path)
                })
            })();
            let _ = sender.send(result);
            repaint();
        });
    }

    pub fn finish_install(&mut self) -> bool {
        let Some(receiver) = &self.installing else {
            return false;
        };
        let Ok(result) = receiver.try_recv() else {
            return false;
        };
        self.installing = None;
        match result.and_then(start_helper) {
            Ok(()) => true,
            Err(error) => {
                self.status = format!("Update failed: {error}");
                false
            }
        }
    }

    pub fn poll(&mut self) {
        if let Some(receiver) = &self.pending {
            if let Ok(release) = receiver.try_recv() {
                self.status = if release.is_some() {
                    "You are up to date."
                } else {
                    "Could not check for updates. Check your connection and repository access. Private repositories require REDOX_UPDATE_TOKEN."
                }.into();
                self.available = release.filter(|release| {
                    newer(&release.tag_name, env!("CARGO_PKG_VERSION"))
                        && release.html_url.starts_with("https://github.com/")
                });
                if self.available.is_some() {
                    self.status = "An update is available.".into();
                }
                self.pending = None;
            }
        }
    }
}

pub fn newer(candidate: &str, current: &str) -> bool {
    fn version(value: &str) -> Option<Vec<u64>> {
        let parts: Option<Vec<_>> = value
            .trim_start_matches('v')
            .split('.')
            .map(|part| part.parse().ok())
            .collect();
        parts.filter(|parts| parts.len() == 3)
    }
    matches!((version(candidate), version(current)), (Some(a), Some(b)) if a > b)
}

fn start_helper(download: std::path::PathBuf) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let target = std::env::current_exe().map_err(|e| e.to_string())?;
        let script = download.with_file_name("update.ps1");
        let quote = |p: &std::path::Path| format!("'{}'", p.to_string_lossy().replace("'", "''"));
        let backup = download.with_file_name("redox-previous.exe");
        let content = format!(
            r#"$ErrorActionPreference = 'Stop'
$target = {target}
$download = {download}
$backup = {backup}
try {{
    Wait-Process -Id {pid} -ErrorAction SilentlyContinue
    $moved = $false
    for ($attempt = 0; $attempt -lt 60; $attempt++) {{
        try {{ Move-Item -LiteralPath $target -Destination $backup; $moved = $true; break }} catch {{ Start-Sleep -Milliseconds 500 }}
    }}
    if (!$moved) {{ throw 'Could not replace the running executable.' }}
    try {{ Move-Item -LiteralPath $download -Destination $target }} catch {{ Move-Item -LiteralPath $backup -Destination $target; throw }}
    Start-Process -FilePath $target -WorkingDirectory (Split-Path -LiteralPath $target)
}} catch {{
    $_ | Out-File -LiteralPath (Join-Path $PSScriptRoot 'update-error.txt')
    if (Test-Path -LiteralPath $target) {{ Start-Process -FilePath $target }}
}}
"#,
            target = quote(&target),
            download = quote(&download),
            backup = quote(&backup),
            pid = std::process::id()
        );
        std::fs::write(&script, content).map_err(|e| e.to_string())?;
        std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-File",
            ])
            .arg(script)
            .creation_flags(0x08000000)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(windows))]
    {
        let _ = download;
        Err("Windows only".into())
    }
}
