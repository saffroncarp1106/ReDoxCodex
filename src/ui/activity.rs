use eframe::egui::{self, Color32, RichText, TextFormat, text::LayoutJob};
use redox::activity::{CommandActivity, FileDiff, TurnActivity};

use super::{scroll::ScrollArea, theme};

const ADDED: Color32 = Color32::from_rgb(111, 191, 128);
const REMOVED: Color32 = Color32::from_rgb(241, 115, 111);

pub(super) fn command(ui: &mut egui::Ui, command: &CommandActivity, streaming: bool) {
    let status = match command.status.as_str() {
        "inProgress" if streaming => "Running".to_owned(),
        "inProgress" => "Stopped".to_owned(),
        "completed" if command.exit_code.unwrap_or(0) == 0 => "Ran".to_owned(),
        "declined" => "Declined".to_owned(),
        _ => command.exit_code.map_or_else(
            || "Command failed".to_owned(),
            |code| format!("Exit {code}"),
        ),
    };
    ui.horizontal_wrapped(|ui| {
        ui.label(
            RichText::new(format!(">_  {status}"))
                .small()
                .color(theme::muted()),
        );
        ui.label(
            RichText::new(&command.command)
                .monospace()
                .small()
                .color(theme::muted()),
        );
    });
}

pub(super) fn changes(ui: &mut egui::Ui, activity: &TurnActivity) {
    if activity.files.is_empty() && !activity.preview_limited {
        return;
    }
    let context = ui.ctx().clone();
    let id = ui.id().with(("changes", context.viewport_id()));
    let mut open = context
        .data(|data| data.get_temp::<bool>(id))
        .unwrap_or(false);
    let added: usize = activity.files.iter().map(|file| file.added).sum();
    let removed: usize = activity.files.iter().map(|file| file.removed).sum();
    let count = activity.files.len();
    let heading = match count {
        0 => "Changes".to_owned(),
        1 => "Edited 1 file".to_owned(),
        _ => format!("Edited {count} files"),
    };
    egui::Frame::new()
        .fill(theme::surface())
        .corner_radius(10)
        .inner_margin(egui::Margin::same(10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.strong(heading);
                counts(ui, added, removed);
                if ui.button("View changes").clicked() {
                    open = true;
                }
            });
            for file in activity.files.iter().take(8) {
                ui.horizontal(|ui| {
                    counts(ui, file.added, file.removed);
                    ui.add(egui::Label::new(&file.path).truncate())
                        .on_hover_text(&file.path);
                });
            }
            if count > 8 {
                ui.label(RichText::new(format!("{} more files", count - 8)).small());
            }
        });
    if open {
        egui::Window::new("Changes · This response")
            .id(id)
            .open(&mut open)
            .default_size([760.0, 500.0])
            .collapsible(false)
            .show(&context, |ui| {
                ui.horizontal(|ui| {
                    ui.strong("This response");
                    counts(ui, added, removed);
                });
                if activity.preview_limited {
                    ui.label(
                        "This diff exceeds the preview limit. Counts reflect the visible portion.",
                    );
                }
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| {
                    for (index, file) in activity.files.iter().enumerate() {
                        egui::CollapsingHeader::new(format!(
                            "{}   +{} −{}",
                            file.path, file.added, file.removed,
                        ))
                        .id_salt(index)
                        .default_open(index == 0)
                        .show(ui, |ui| patch(ui, file));
                    }
                });
            });
    }
    context.data_mut(|data| data.insert_temp(id, open));
}

fn counts(ui: &mut egui::Ui, added: usize, removed: usize) {
    ui.label(RichText::new(format!("+{added}")).color(ADDED));
    ui.label(RichText::new(format!("−{removed}")).color(REMOVED));
}

fn patch(ui: &mut egui::Ui, file: &FileDiff) {
    ScrollArea::horizontal().show(ui, |ui| {
        let mut job = LayoutJob::default();
        let mut in_hunk = false;
        for line in file.patch.split_inclusive('\n') {
            if line.starts_with("@@ ") {
                in_hunk = true;
            }
            let (color, background) = match line.as_bytes().first() {
                Some(b'+') if in_hunk => (ADDED, Color32::from_rgb(27, 54, 37)),
                Some(b'-') if in_hunk => (REMOVED, Color32::from_rgb(63, 34, 36)),
                Some(b'@') => (theme::muted(), Color32::TRANSPARENT),
                _ => (theme::text(), Color32::TRANSPARENT),
            };
            job.append(
                line,
                0.0,
                TextFormat {
                    font_id: egui::FontId::monospace(12.0),
                    color,
                    background,
                    ..Default::default()
                },
            );
        }
        ui.label(job);
    });
}
