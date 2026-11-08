use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_COMMANDS: usize = 96;
const MAX_DIFF_BYTES: usize = 256 * 1024;

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct TurnActivity {
    pub turn_id: String,
    pub commands: Vec<CommandActivity>,
    pub files: Vec<FileDiff>,
    pub preview_limited: bool,
}

impl TurnActivity {
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty() && self.files.is_empty() && !self.preview_limited
    }

    pub fn record_command(&mut self, command: CommandActivity) {
        if let Some(existing) = self.commands.iter_mut().find(|item| item.id == command.id) {
            existing.status = command.status;
            existing.exit_code = command.exit_code;
        } else if self.commands.len() < MAX_COMMANDS {
            self.commands.push(command);
        }
    }

    pub fn replace_diff(&mut self, diff: &str) {
        self.preview_limited = diff.len() > MAX_DIFF_BYTES;
        let diff = if self.preview_limited {
            let end = diff.as_bytes()[..MAX_DIFF_BYTES]
                .iter()
                .rposition(|byte| *byte == b'\n')
                .map_or(0, |index| index + 1);
            &diff[..end]
        } else {
            diff
        };
        self.files.clear();
        for (start, _) in diff.match_indices("diff --git ") {
            if start != 0 && diff.as_bytes()[start - 1] != b'\n' {
                continue;
            }
            let section = &diff[start..];
            let end = section
                .find("\ndiff --git ")
                .map_or(section.len(), |index| index + 1);
            if let Some(file) = FileDiff::parse(&section[..end]) {
                self.files.push(file);
            }
        }
        if self.files.is_empty()
            && let Some(file) = FileDiff::parse(diff)
        {
            self.files.push(file);
        }
    }
}

pub enum ActivityUpdate {
    Command(CommandActivity),
    Diff(String),
}

#[derive(Clone, Deserialize, Serialize)]
pub struct CommandActivity {
    id: String,
    pub offset: usize,
    pub command: String,
    pub status: String,
    pub exit_code: Option<i64>,
}

impl CommandActivity {
    pub fn from_item(item: &Value) -> Option<Self> {
        if item["type"] != "commandExecution" {
            return None;
        }
        Some(Self {
            id: item["id"].as_str()?.to_owned(),
            offset: 0,
            command: item["command"].as_str()?.chars().take(2_048).collect(),
            status: item["status"].as_str().unwrap_or("inProgress").to_owned(),
            exit_code: item["exitCode"].as_i64(),
        })
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub patch: String,
    pub added: usize,
    pub removed: usize,
}

impl FileDiff {
    fn parse(patch: &str) -> Option<Self> {
        let mut before = None;
        let mut after = None;
        let mut in_hunk = false;
        let mut added = 0;
        let mut removed = 0;
        for line in patch.lines() {
            if line.starts_with("@@ ") {
                in_hunk = true;
            } else if in_hunk {
                added += usize::from(line.starts_with('+'));
                removed += usize::from(line.starts_with('-'));
            } else if let Some(path) = line.strip_prefix("--- ") {
                before = patch_path(path, "a/");
            } else if let Some(path) = line.strip_prefix("+++ ") {
                after = patch_path(path, "b/");
            } else if let Some(path) = line.strip_prefix("rename to ") {
                after = patch_path(path, "");
            }
        }
        let path = after.or(before).or_else(|| {
            let header = patch.lines().next()?.strip_prefix("diff --git ")?;
            let (_, path) = header.rsplit_once(" b/")?;
            patch_path(path, "")
        })?;
        Some(Self {
            path,
            patch: patch.to_owned(),
            added,
            removed,
        })
    }
}

fn patch_path(path: &str, prefix: &str) -> Option<String> {
    let path = path.split('\t').next()?.trim_end_matches('\r');
    let path =
        serde_json::from_str::<String>(path).unwrap_or_else(|_| path.trim_matches('"').to_owned());
    if path.is_empty() || path == "/dev/null" {
        return None;
    }
    Some(path.strip_prefix(prefix).unwrap_or(&path).to_owned())
}
