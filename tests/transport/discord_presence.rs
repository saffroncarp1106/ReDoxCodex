use super::*;

#[derive(Default)]
struct Fake {
    connects: usize,
    clears: usize,
    sent: Vec<(Settings, i64)>,
    fail: bool,
}
impl Transport for Fake {
    fn connect(&mut self) -> Result<(), String> {
        self.connects += 1;
        Ok(())
    }
    fn publish(&mut self, settings: &Settings, started: i64) -> Result<(), String> {
        if self.fail {
            return Err("disconnected".into());
        }
        self.sent.push((settings.clone(), started));
        Ok(())
    }
    fn disconnect(&mut self) {
        self.clears += 1;
    }
}

#[test]
fn disabled_is_quiet_and_disabling_clears_the_existing_connection() {
    let mut engine = Engine::new(Fake::default());
    let mut settings = Settings::default();
    assert_eq!(engine.update(&settings), "Disabled");
    assert_eq!(engine.transport.connects, 0);
    settings.enabled = true;
    engine.update(&settings);
    settings.state = "Coding".into();
    engine.update(&settings);
    assert_eq!(engine.transport.connects, 1);
    assert_eq!(engine.transport.sent[1].0.state, "Coding");
    assert_eq!(engine.transport.sent[0].1, engine.transport.sent[1].1);
    settings.enabled = false;
    engine.update(&settings);
    engine.update(&settings);
    assert_eq!(engine.transport.clears, 1);
}

#[test]
fn broken_connections_retry_with_the_latest_text() {
    let mut engine = Engine::new(Fake {
        fail: true,
        ..Default::default()
    });
    let mut settings = Settings {
        enabled: true,
        ..Default::default()
    };
    assert!(engine.update(&settings).contains("retrying"));
    assert!(!engine.connected);
    engine.transport.fail = false;
    settings.details = "Custom activity".into();
    assert_eq!(engine.update(&settings), "Presence sent to Discord");
    assert_eq!(engine.transport.connects, 2);
    assert_eq!(engine.transport.sent[0].0.details, "Custom activity");
}

#[test]
fn text_is_bounded_and_settings_survive_serialization() {
    let settings = Settings {
        enabled: true,
        details: "  ".into(),
        state: "a".repeat(200),
    };
    let restored: Settings =
        serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
    assert_eq!(restored, settings);
    let normalized = restored.normalized();
    assert_eq!(normalized.details, "ReDox");
    assert_eq!(normalized.state.len(), 128);
    let old: Settings = serde_json::from_str("{}").unwrap();
    assert!(!old.enabled);
}
