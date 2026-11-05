use super::theme::{self, muted};
use crate::app::ForgeDesk;
use eframe::egui::{self, Align, Align2, Color32, Layout, RichText, vec2};

impl ForgeDesk {
    pub(in crate::app) fn usage_dialog(&mut self, context: &egui::Context) {
        let mut open = self.usage_open;
        egui::Window::new("Usage limits").open(&mut open).anchor(Align2::LEFT_BOTTOM, vec2(54.0, -64.0)).default_width(380.0).resizable(false).collapsible(false).show(context, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(&self.account).color(muted()));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.add_enabled(!self.refreshing_usage, egui::Button::new("Refresh")).clicked() { self.refresh_usage(context); }
                    if self.refreshing_usage { ui.spinner(); }
                });
            });
            if let Some(error) = &self.usage_error {
                ui.label(RichText::new(error).color(Color32::from_rgb(235,150,135)));
                if self.usage.is_some() { ui.label(RichText::new("Showing the last successful update.").small().color(muted())); }
            }
            if let Some(info) = &self.usage {
                if info.ordinary_usage_allowed == Some(false) {
                    ui.label(RichText::new("An account usage limit has been reached.").color(Color32::from_rgb(235,150,135)));
                }
                if info.buckets.is_empty() { ui.label("No quota windows were returned for this account."); }
                egui::ScrollArea::vertical().max_height((context.content_rect().height()-280.0).clamp(160.0,450.0)).show(ui, |ui| {
                    for bucket in &info.buckets {
                        ui.add_space(12.0);
                        let title = bucket.limit_name.as_deref().or(bucket.limit_id.as_deref()).unwrap_or("Account quota");
                        ui.label(RichText::new(title).font(theme::semibold(15.0)));
                        if let Some(plan) = &bucket.plan_type { ui.label(RichText::new(format!("Plan: {plan}")).small().color(muted())); }
                        if bucket.rate_limit_reached_type.is_some() { ui.label("Limit reached"); }
                        for window in [&bucket.primary, &bucket.secondary].into_iter().flatten() {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(window.label());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| { ui.label(format!("{:.0}% remaining", window.remaining_percent())); });
                            });
                            ui.add(egui::ProgressBar::new((window.used_percent / 100.0).clamp(0.0,1.0) as f32)
                                .fill(theme::blue()).text(format!("{:.0}% used", window.used_percent)));
                            let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |time| time.as_secs() as i64);
                            ui.label(RichText::new(window.reset_label(now)).small().color(muted()));
                        }
                        if bucket.primary.is_none() && bucket.secondary.is_none() { ui.label("Window details not provided."); }
                        if let Some(credits) = &bucket.credits {
                            if credits.unlimited { ui.label("Credits: unlimited"); }
                            else if let Some(balance) = &credits.balance { ui.label(format!("Credit balance: {balance}")); }
                        }
                        ui.add_space(8.0);
                        ui.separator();
                    }
                });
                if let Some(count) = info.available_resets { ui.label(format!("Available quota resets: {count}")); }
            } else if !self.refreshing_usage && self.usage_error.is_none() {
                ui.label("Refresh to read your account's limits.");
            }
            ui.add_space(10.0);
            if let Some(updated) = self.usage_updated {
                ui.label(RichText::new(format!("Updated {} seconds ago", updated.elapsed().as_secs())).small().color(muted()));
            }
            ui.label(RichText::new("Limits reported by Codex for this account. ChatGPT website limits may differ.").small().color(muted()));
        });
        self.usage_open = open;
        if open {
            context.request_repaint_after(std::time::Duration::from_secs(15));
        }
    }
}
