use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use discord_rich_presence::{DiscordIpc, DiscordIpcClient, activity};
use serde::{Deserialize, Serialize};

const APPLICATION_ID: &str = "1569148102085185808";

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub details: String,
    pub state: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            details: "ReDox".into(),
            state: "Vibing".into(),
        }
    }
}

impl Settings {
    fn normalized(&self) -> Self {
        let text = |value: &str, fallback: &str| {
            let text: String = value
                .trim()
                .chars()
                .filter(|c| !c.is_control())
                .take(128)
                .collect();
            if text.is_empty() {
                fallback.to_owned()
            } else {
                text
            }
        };
        Self {
            enabled: self.enabled,
            details: text(&self.details, "ReDox"),
            state: text(&self.state, "Vibing"),
        }
    }
}

pub struct Presence {
    sender: mpsc::Sender<Option<Settings>>,
    status: Arc<Mutex<String>>,
    last: Settings,
    finished: mpsc::Receiver<()>,
}

impl Presence {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel();
        let (done, finished) = mpsc::channel();
        let status = Arc::new(Mutex::new("Disabled".into()));
        let worker_status = Arc::clone(&status);
        let spawned = std::thread::Builder::new()
            .name("discord-presence".into())
            .spawn(move || {
                let mut engine = Engine::new(DiscordTransport(None));
                let mut settings = Settings::default();
                'worker: loop {
                    let timeout = if settings.enabled {
                        Duration::from_secs(15)
                    } else {
                        Duration::from_secs(3600)
                    };
                    match receiver.recv_timeout(timeout) {
                        Ok(Some(next)) => settings = next,
                        Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => {}
                    }
                    let mut stop = false;
                    while let Ok(command) = receiver.try_recv() {
                        match command {
                            Some(next) => settings = next,
                            None => {
                                stop = true;
                                break;
                            }
                        }
                    }
                    if stop {
                        break;
                    }
                    if settings.enabled {
                        loop {
                            match receiver.recv_timeout(Duration::from_millis(400)) {
                                Ok(Some(next)) => {
                                    settings = next;
                                    if !settings.enabled {
                                        break;
                                    }
                                }
                                Ok(None) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                                    break 'worker;
                                }
                                Err(mpsc::RecvTimeoutError::Timeout) => break,
                            }
                        }
                    }
                    let message = engine.update(&settings);
                    if let Ok(mut status) = worker_status.lock() {
                        *status = message;
                    }
                }
                engine.transport.disconnect();
                let _ = done.send(());
            });
        if let Err(error) = spawned {
            if let Ok(mut status) = status.lock() {
                *status = format!("Could not start presence worker: {error}");
            }
        }
        Self {
            sender,
            status,
            last: Settings::default(),
            finished,
        }
    }

    pub fn sync(&mut self, settings: &Settings) {
        let settings = settings.normalized();
        if settings != self.last {
            self.last = settings.clone();
            let _ = self.sender.send(Some(settings));
        }
    }

    pub fn status(&self) -> String {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|_| "Unavailable".into())
    }
}

impl Drop for Presence {
    fn drop(&mut self) {
        let _ = self.sender.send(None);
        let _ = self.finished.recv_timeout(Duration::from_millis(500));
    }
}

trait Transport {
    fn connect(&mut self) -> Result<(), String>;
    fn publish(&mut self, settings: &Settings, started: i64) -> Result<(), String>;
    fn disconnect(&mut self);
}

struct DiscordTransport(Option<DiscordIpcClient>);

impl Transport for DiscordTransport {
    fn connect(&mut self) -> Result<(), String> {
        let mut client = DiscordIpcClient::new(APPLICATION_ID);
        client.connect().map_err(|error| error.to_string())?;
        self.0 = Some(client);
        Ok(())
    }
    fn publish(&mut self, settings: &Settings, started: i64) -> Result<(), String> {
        self.0
            .as_mut()
            .ok_or("Discord disconnected")?
            .set_activity(
                activity::Activity::new()
                    .details(&settings.details)
                    .state(&settings.state)
                    .timestamps(activity::Timestamps::new().start(started)),
            )
            .map_err(|error| error.to_string())
    }
    fn disconnect(&mut self) {
        if let Some(mut client) = self.0.take() {
            let _ = client.clear_activity();
            let _ = client.close();
        }
    }
}

struct Engine<T> {
    transport: T,
    connected: bool,
    started: i64,
}

impl<T: Transport> Engine<T> {
    fn new(transport: T) -> Self {
        Self {
            transport,
            connected: false,
            started: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |time| time.as_secs() as i64),
        }
    }
    fn update(&mut self, settings: &Settings) -> String {
        if !settings.enabled {
            if self.connected {
                self.transport.disconnect();
            }
            self.connected = false;
            return "Disabled".into();
        }
        let result = (|| {
            if !self.connected {
                self.transport.connect()?;
                self.connected = true;
            }
            self.transport.publish(&settings.normalized(), self.started)
        })();
        match result {
            Ok(()) => "Presence sent to Discord".into(),
            Err(error) => {
                self.transport.disconnect();
                self.connected = false;
                format!("Discord unavailable; retrying within 15 seconds: {error}")
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/transport/discord_presence.rs"]
mod tests;
