use crate::app::ReDox;
use eframe::egui;
use redox_studio::actions::UiAction;

impl ReDox {
    pub(crate) fn dispatch_ui_action(&mut self, action: UiAction, ctx: &egui::Context) {
        if let Some(scene) = action.scene() {
            self.settings_page = match scene {
                redox_studio::scenes::SceneId::Models => crate::app::ui::SettingsPage::Models,
                redox_studio::scenes::SceneId::Workspace => crate::app::ui::SettingsPage::Workspace,
                redox_studio::scenes::SceneId::Appearance => {
                    crate::app::ui::SettingsPage::Appearance
                }
                redox_studio::scenes::SceneId::Cursors => crate::app::ui::SettingsPage::Cursors,
                redox_studio::scenes::SceneId::Advanced => crate::app::ui::SettingsPage::Advanced,
                redox_studio::scenes::SceneId::Updates => crate::app::ui::SettingsPage::Updates,
                redox_studio::scenes::SceneId::About => crate::app::ui::SettingsPage::About,
                _ => crate::app::ui::SettingsPage::Account,
            };
            self.settings_open = action != UiAction::SettingsClose;
            return;
        }
        match action {
            UiAction::History => {
                self.sidebar_visible = true;
                self.refresh_history(ctx, false);
            }
            UiAction::Chats => {
                self.sidebar_visible = true;
                self.search_open = false;
            }
            UiAction::Project | UiAction::OpenProject => self.pick_workspace(ctx),
            UiAction::NewChat => {
                if self.can_new_chat() {
                    self.new_chat();
                }
            }
            UiAction::Send => self.send_prompt(ctx),
            UiAction::Stop => self.stop_response(),
            UiAction::Previous => self.navigate(-1),
            UiAction::Next => self.navigate(1),
            UiAction::Rename => self.rename_open = true,
            UiAction::OllamaAgent => {
                self.ui_preferences.agents_open = !self.ui_preferences.agents_open
            }
            UiAction::Settings
            | UiAction::SettingsAccount
            | UiAction::SettingsModels
            | UiAction::SettingsWorkspace
            | UiAction::SettingsAppearance
            | UiAction::SettingsCursors
            | UiAction::SettingsAdvanced
            | UiAction::SettingsUpdates
            | UiAction::SettingsAbout
            | UiAction::SettingsClose => {}
            UiAction::Sidebar => self.sidebar_visible = !self.sidebar_visible,
            UiAction::AddFiles => self.pick_files(ctx),
            UiAction::Menus
            | UiAction::Model
            | UiAction::Effort
            | UiAction::ModelEffort
            | UiAction::Composer
            | UiAction::Conversation => {}
        }
    }
}
