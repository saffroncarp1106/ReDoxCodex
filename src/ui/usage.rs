use super::theme::{self, muted};
use crate::app::ReDox;
use eframe::egui::{self, Align, Align2, Color32, Layout, RichText, vec2};

impl ReDox {
    pub(super) fn sidebar_usage(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.add_space(8.0);
        if self.is_ollama() {
            if let Some(chat) = self.session.active() {
                let stats = &chat.ollama_usage.last;
                let limit = self.session.ollama.context_tokens.max(1);
                ui.label(
                    RichText::new(format!(
                        "Context: {:.0}% used",
                        stats.prompt_tokens as f64 / limit as f64 * 100.0
                    ))
                    .small(),
                );
                ui.label(
                    RichText::new(format!(
                        "{} input / {} output tokens",
                        stats.prompt_tokens, stats.output_tokens
                    ))
                    .small(),
                );
            }
        } else {
            let mut available = false;
            if let Some(info) = &self.usage {
                for bucket in &info.buckets {
                    for window in [&bucket.primary, &bucket.secondary].into_iter().flatten() {
                        available = true;
                        ui.label(
                            RichText::new(format!(
                                "{}: {:.0}% remaining",
                                window.label(),
                                window.remaining_percent()
                            ))
                            .small(),
                        );
                    }
                }
            }
            if !available {
                ui.label(
                    RichText::new("Usage percentages not loaded")
                        .small()
                        .color(muted()),
                );
            }
        }
        ui.add_space(6.0);
        if ui.button("Usage details / refresh").clicked() {
            self.open_usage(context);
        }
        ui.add_space(8.0);
    }
    pub(in crate::app) fn usage_dialog(&mut self, context: &egui::Context) {
        if self.active_ui.config.layout == "win98" && self.usage_open {
            let mut open = true;
            egui::Window::new("Usage limits")
                .title_bar(false)
                .open(&mut open)
                .default_width(420.0)
                .show(context, |ui| {
                    let (rect, _) = ui
                        .allocate_exact_size(vec2(ui.available_width(), 24.0), egui::Sense::drag());
                    super::win98::caption_background(ui.painter(), rect);
                    ui.painter().text(
                        rect.left_center() + vec2(6.0, 0.0),
                        Align2::LEFT_CENTER,
                        "Usage limits",
                        theme::semibold(14.0),
                        Color32::WHITE,
                    );
                    let close_rect = egui::Rect::from_center_size(
                        rect.right_center() - vec2(12.0, 0.0),
                        vec2(19.0, 19.0),
                    );
                    let close = ui.put(close_rect, egui::Button::new("X"));
                    super::win98::bevel(
                        ui.painter(),
                        close.rect,
                        close.is_pointer_button_down_on(),
                    );
                    if close.clicked() {
                        self.usage_open = false;
                    }
                    egui::Frame::NONE.inner_margin(8).show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
                        self.usage_content(ui, context);
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if super::win98::button(ui, "OK", vec2(78.0, 25.0)).clicked() {
                                    self.usage_open = false;
                                }
                            });
                        });
                    });
                });
            self.usage_open &= open;
            return;
        }
        let mut open = self.usage_open;
        if self.ui_preferences.floating_windows {
            super::window_frame::native_popup(
                context,
                "native_usage",
                "Usage limits",
                vec2(420.0, 580.0),
                &mut open,
                |ui| {
                    let child = ui.ctx().clone();
                    self.usage_content(ui, &child);
                },
            );
            self.usage_open = open;
            if open {
                context.request_repaint_after(std::time::Duration::from_secs(15));
            }
            return;
        }
        egui::Window::new("Usage limits")
            .open(&mut open)
            .anchor(Align2::LEFT_BOTTOM, vec2(54.0, -64.0))
            .default_width(380.0)
            .resizable(false)
            .collapsible(false)
            .show(context, |ui| {
                self.usage_content(ui, context);
            });
        self.usage_open = open;
        if open {
            context.request_repaint_after(std::time::Duration::from_secs(15));
        }
    }

    fn usage_content(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        let id = egui::Id::new("usage_limits_tab");
        let mut about = context
            .data(|data| data.get_temp::<bool>(id))
            .unwrap_or(false);
        let classic = self.active_ui.config.layout == super::win98::ID;
        ui.spacing_mut().item_spacing = vec2(8.0, 0.0);
        let mut selected_tab = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 1.0;
            for (label, value) in [("Usage", false), ("About", true)] {
                let response = ui.add(
                    egui::Button::new(label)
                        .fill(if about == value {
                            theme::sidebar()
                        } else {
                            Color32::from_gray(176)
                        })
                        .min_size(vec2(74.0, 24.0)),
                );
                if classic {
                    super::win98::bevel(ui.painter(), response.rect, false);
                }
                if about == value {
                    selected_tab = Some(response.rect);
                }
                if response.clicked() {
                    about = value;
                }
            }
        });
        context.data_mut(|data| data.insert_temp(id, about));
        let pane = egui::Frame::NONE.inner_margin(12).show(ui, |ui| {
            ui.set_min_height(280.0);
            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
            if about {
                ui.heading("About usage limits");
                ui.label("Usage limits and reset times are reported by your account.");
                ui.add_space(8.0);
                ui.label("ReDox does not provide a reset limits button or spend credits to reset quotas. This avoids accidental credit use or unintended reset requests.");
                ui.add_space(8.0);
                ui.label("Use the send timer to queue a request for after your hourly or weekly limit resets. Refresh only reads the latest usage information.");
            } else {
                self.usage_details(ui, context);
            }
        });
        if classic {
            super::win98::bevel(ui.painter(), pane.response.rect, false);
            if let Some(tab) = selected_tab {
                let join = egui::Rect::from_min_max(
                    tab.left_bottom() + vec2(1.0, -1.0),
                    tab.right_bottom() + vec2(-1.0, 1.5),
                );
                ui.painter().rect_filled(join, 0, theme::sidebar());
            }
        }
    }

    fn usage_details(&mut self, ui: &mut egui::Ui, context: &egui::Context) {
        ui.horizontal(|ui| {
            ui.label(RichText::new(&self.account).color(muted()));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add_enabled(!self.refreshing_usage, egui::Button::new("Refresh"))
                    .clicked()
                {
                    self.refresh_usage(context);
                }
                if self.refreshing_usage {
                    ui.spinner();
                }
            });
        });
        if let Some(error) = &self.usage_error {
            ui.label(RichText::new(error).color(Color32::from_rgb(235, 150, 135)));
            if self.usage.is_some() {
                ui.label(
                    RichText::new("Showing the last successful update.")
                        .small()
                        .color(muted()),
                );
            }
        }
        if let Some(info) = &self.usage {
            if info.ordinary_usage_allowed == Some(false) {
                ui.label(
                    RichText::new("An account usage limit has been reached.")
                        .color(Color32::from_rgb(235, 150, 135)),
                );
            }
            if info.buckets.is_empty() {
                ui.label("No quota windows were returned for this account.");
            }
            super::scroll::ScrollArea::vertical()
                .max_height((context.content_rect().height() - 280.0).clamp(160.0, 450.0))
                .show(ui, |ui| {
                    for bucket in &info.buckets {
                        ui.add_space(12.0);
                        let title = bucket
                            .limit_name
                            .as_deref()
                            .or(bucket.limit_id.as_deref())
                            .unwrap_or("Account quota");
                        ui.label(RichText::new(title).font(theme::semibold(15.0)));
                        if let Some(plan) = &bucket.plan_type {
                            ui.label(
                                RichText::new(format!("Plan: {plan}"))
                                    .small()
                                    .color(muted()),
                            );
                        }
                        if bucket.rate_limit_reached_type.is_some() {
                            ui.label("Limit reached");
                        }
                        for window in [&bucket.primary, &bucket.secondary].into_iter().flatten() {
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(window.label());
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(format!(
                                        "{:.0}% remaining",
                                        window.remaining_percent()
                                    ));
                                });
                            });
                            ui.add(
                                egui::ProgressBar::new((window.remaining_percent() / 100.0) as f32)
                                    .corner_radius(0)
                                    .fill(theme::blue()),
                            );
                            let now = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map_or(0, |time| time.as_secs() as i64);
                            ui.label(
                                RichText::new(window.reset_label(now))
                                    .small()
                                    .color(muted()),
                            );
                        }
                        if bucket.primary.is_none() && bucket.secondary.is_none() {
                            ui.label("Window details not provided.");
                        }
                        if let Some(credits) = &bucket.credits {
                            if credits.unlimited {
                                ui.label("Credits: unlimited");
                            } else if let Some(balance) = &credits.balance {
                                ui.label(format!("Credit balance: {balance}"));
                            }
                        }
                        ui.add_space(8.0);
                        ui.separator();
                    }
                });
            if let Some(count) = info.available_resets {
                ui.label(format!("Available quota resets: {count}"));
            }
        } else if !self.refreshing_usage && self.usage_error.is_none() {
            ui.label("Refresh to read your account's limits.");
        }
        ui.add_space(10.0);
        if let Some(updated) = self.usage_updated {
            ui.label(
                RichText::new(format!(
                    "Updated {} seconds ago",
                    updated.elapsed().as_secs()
                ))
                .small()
                .color(muted()),
            );
        }
    }
}
