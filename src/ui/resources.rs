use crate::app::ReDox;
use eframe::egui::{self, RichText};

impl ReDox {
    pub(super) fn resource_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Resource limiter");
        ui.label("Shared budgets for ReDox and its child processes, including the Codex connection and commands launched by assistants. Budgets stay active while enabled, including between prompts.");
        ui.label("Existing external services, separately running tunnels, and Ollama servers use their own resources.");
        ui.separator();

        let settings = &mut self.resource_draft;
        ui.checkbox(&mut settings.enabled, "Enable resource budgets");
        ui.add_enabled_ui(settings.enabled && cfg!(windows), |ui| {
            resource_bar(ui, "CPU", &mut settings.cpu_percent);
            resource_bar(ui, "RAM", &mut settings.ram_percent);
            ui.label("RAM sets the combined committed-memory ceiling as a percentage of installed physical RAM. Reaching it can cause allocations and tasks to fail, including ReDox itself.");
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Connection upload capacity");
                ui.add(egui::DragValue::new(&mut settings.upload_mbps).range(1..=100_000).suffix(" Mbps"));
            });
            resource_bar(ui, "Upload", &mut settings.upload_percent);
            ui.label(format!("Upload ceiling {:.2} Mbps. Enter your connection's upload speed above. The cap covers outgoing traffic across Wi-Fi and Ethernet.", settings.upload_bytes_per_second() as f64 * 8.0 / 1_000_000.0));
        });
        ui.add_space(8.0);
        ui.label(RichText::new("Additional coverage").strong());
        ui.label("GPU percentage enforcement is unavailable in this implementation.");
        ui.label("Disk speed enforcement is unavailable. Windows retired the job-object disk-rate API on current versions.");
        ui.label("Network download throttling is unavailable in this implementation.");
        ui.add_space(8.0);
        if ui.button("Apply resource budgets").clicked() {
            settings.normalize();
            self.resource_limiter.apply(settings);
            self.session.resources = settings.clone();
        }
        ui.label("Apply updates the running app immediately. Saved budgets are restored on startup. Status below reflects the last applied values.");
        for status in &self.resource_limiter.status {
            ui.label(status);
        }
    }
}

fn resource_bar(ui: &mut egui::Ui, label: &str, percent: &mut u32) {
    ui.horizontal(|ui| {
        let label = ui.add_sized([60.0, 18.0], egui::Label::new(label));
        ui.spacing_mut().slider_width = (ui.available_width() - 64.0).clamp(24.0, 140.0);
        ui.spacing_mut().slider_rail_height = 4.0;
        ui.spacing_mut().interact_size.y = 18.0;
        ui.add(
            egui::Slider::new(percent, 1..=100)
                .show_value(false)
                .trailing_fill(true),
        )
        .labelled_by(label.id);
        ui.add(egui::DragValue::new(percent).range(1..=100).suffix("%"))
            .labelled_by(label.id);
    });
}
