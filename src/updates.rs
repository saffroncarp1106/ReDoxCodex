use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

#[derive(serde::Deserialize)]
pub struct Release {
    pub tag_name: String,
    pub html_url: String,
}

#[derive(Default)]
pub struct Updates {
    started: bool,
    pending: Option<Receiver<Option<Release>>>,
    pub available: Option<Release>,
}

impl Updates {
    pub fn poll(&mut self, repaint: impl FnOnce() + Send + 'static) {
        if !self.started {
            self.started = true;
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
        if let Some(receiver) = &self.pending {
            if let Ok(release) = receiver.try_recv() {
                self.available = release.filter(|release| {
                    newer(&release.tag_name, env!("CARGO_PKG_VERSION"))
                        && release.html_url.starts_with("https://github.com/")
                });
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
