use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use forgedesk::attachments::{Attachment, MAX_ATTACHMENTS};

use forgedesk::codex::{
    self, Approval, ChatRequest, ConnectionInfo, LoginChallenge, LoginEvent, ModelInfo,
};
use forgedesk::domain::{DEFAULT_MODEL, MessageStatus, Session};
use forgedesk::history::{self, ConversationPage, HistoryEntry, HistoryPage};
use forgedesk::run::{self, RunEvent, Running, Update};
use forgedesk::usage::UsageInfo;

#[path = "ui/mod.rs"]
mod ui;

use ui::theme;

const STORAGE_KEY: &str = "forgedesk.session.v1";

enum Event {
    UiImport(Result<String, String>),
    History(u64, bool, Result<HistoryPage, String>),
    Conversation(
        String,
        Option<uuid::Uuid>,
        bool,
        Result<ConversationPage, String>,
    ),
    Models(Result<ConnectionInfo, String>),
    Workspace(Result<Option<PathBuf>, String>),
    Attachments(uuid::Uuid, Result<Vec<Attachment>, String>),
    Usage(Result<UsageInfo, String>),
    CodexExecutable(Option<PathBuf>),
}

pub struct ForgeDesk {
    active_ui: ui::packs::Pack,
    ui_catalog: ui::packs::Catalog,
    ui_folder: PathBuf,
    ui_error: Option<String>,
    importing_ui: bool,
    ui_scanned: Instant,
    ui_background: Option<egui::TextureHandle>,
    restart_request: std::sync::Arc<std::sync::Mutex<bool>>,
    startup: ui::startup::Startup,
    history: Vec<HistoryEntry>,
    history_cursor: Option<String>,
    history_loading: bool,
    history_error: Option<String>,
    history_generation: u64,
    history_filter: (bool, String),
    history_reading: Option<String>,
    conversation_cursors: std::collections::HashMap<String, Option<String>>,
    session: Session,
    sender: SyncSender<Event>,
    receiver: Receiver<Event>,
    models: Vec<String>,
    model_details: Vec<ModelInfo>,
    default_model: Option<String>,
    usage: Option<UsageInfo>,
    usage_error: Option<String>,
    usage_open: bool,
    refreshing_usage: bool,
    usage_updated: Option<Instant>,
    refreshing: bool,
    picking_workspace: bool,
    picking_files: bool,
    attachment_error: Option<String>,
    settings_open: bool,
    show_archived: bool,
    status: String,
    runtime: Option<tokio::runtime::Runtime>,
    run_sender: tokio::sync::mpsc::Sender<RunEvent>,
    run_receiver: tokio::sync::mpsc::Receiver<RunEvent>,
    running: Option<Running>,
    approval: Option<Approval>,
    account: String,
    login_receiver: Option<tokio::sync::mpsc::Receiver<LoginEvent>>,
    login_task: Option<tokio::task::JoinHandle<()>>,
    login_text: String,
    login_challenge: Option<LoginChallenge>,
    signed_in: bool,
    sidebar_visible: bool,
    search_open: bool,
    search_query: String,
    rename_open: bool,
    capture_path: Option<PathBuf>,
    capture_frames: usize,
    ui_preferences: ui::Preferences,
    settings_page: ui::SettingsPage,
}

impl ForgeDesk {
    pub fn new(
        context: &eframe::CreationContext<'_>,
        capture_path: Option<PathBuf>,
        restart_request: std::sync::Arc<std::sync::Mutex<bool>>,
    ) -> Self {
        let mut ui_preferences: ui::Preferences = context
            .storage
            .and_then(|storage| eframe::get_value(storage, ui::PREFERENCES_KEY))
            .unwrap_or_default();
        ui_preferences.normalize();
        let ui_folder = ui::packs::root();
        let ui_catalog = ui::packs::discover(&ui_folder);
        let selected = std::env::args()
            .find_map(|arg| arg.strip_prefix("--ui=").map(str::to_owned))
            .unwrap_or_else(|| ui_preferences.ui.clone());
        let active_ui = ui_catalog
            .packs
            .iter()
            .find(|pack| pack.id == selected)
            .unwrap_or(&ui_catalog.packs[0])
            .clone();
        let ui_error = if active_ui.id != selected {
            Some(format!("UI '{selected}' is unavailable. Using Codex."))
        } else {
            None
        };
        if capture_path.is_none() {
            ui_preferences.ui = active_ui.id.clone();
        }
        theme::set_palette(active_ui.config.colors().unwrap_or_default());
        theme::apply(&context.egui_ctx);
        let ui_background = active_ui
            .config
            .background_image
            .as_deref()
            .and_then(|asset| ui::packs::asset_path(&active_ui.path, asset).ok())
            .and_then(|path| image::open(path).ok())
            .map(|image| {
                let rgba = image.to_rgba8();
                context.egui_ctx.load_texture(
                    "ui_pack_background",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        let sidebar_visible = ui_preferences
            .sidebars
            .get(&active_ui.id)
            .copied()
            .unwrap_or(if active_ui.config.layout == ui::packs::Layout::Codex {
                ui_preferences.sidebar_visible
            } else {
                active_ui.config.sidebar_default
            });
        context.egui_ctx.set_zoom_factor(ui_preferences.zoom);

        let mut session: Session = context
            .storage
            .and_then(|storage| eframe::get_value(storage, STORAGE_KEY))
            .unwrap_or_default();

        session.recover();
        session.codex_path = codex::resolve_executable(&session.codex_path);
        if session.workspace.is_none() {
            session.workspace = std::env::current_dir().ok();
        }
        if session.active_chat.is_none() {
            session.new_chat();
        }

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build();
        let status = match &runtime {
            Ok(_) => "Ready. Choose a model in settings or start a chat.".into(),
            Err(error) => format!("Background runtime unavailable: {error}"),
        };

        let (sender, receiver) = mpsc::sync_channel(4);
        let (run_sender, run_receiver) = tokio::sync::mpsc::channel(128);

        let mut app = Self {
            active_ui,
            ui_catalog,
            ui_folder,
            ui_error,
            importing_ui: false,
            ui_scanned: Instant::now(),
            ui_background,
            restart_request,
            startup: ui::startup::Startup::new(ui_preferences.startup.clone()),
            history: Vec::new(),
            history_cursor: None,
            history_loading: false,
            history_error: None,
            history_generation: 0,
            history_filter: (false, String::new()),
            history_reading: None,
            conversation_cursors: std::collections::HashMap::new(),
            session,
            sender,
            receiver,
            models: Vec::new(),
            model_details: Vec::new(),
            default_model: None,
            usage: None,
            usage_error: None,
            usage_open: false,
            refreshing_usage: false,
            usage_updated: None,
            refreshing: false,
            picking_workspace: false,
            picking_files: false,
            attachment_error: None,
            settings_open: false,
            show_archived: false,
            status,
            runtime: runtime.ok(),
            run_sender,
            run_receiver,
            running: None,
            approval: None,
            account: "Checking ChatGPT account...".into(),
            login_receiver: None,
            login_task: None,
            login_text: String::new(),
            login_challenge: None,
            signed_in: false,
            sidebar_visible,
            search_open: false,
            search_query: String::new(),
            rename_open: false,
            capture_path,
            capture_frames: 0,
            ui_preferences,
            settings_page: ui::SettingsPage::default(),
        };
        app.refresh_models(&context.egui_ctx);
        app
    }

    fn scan_ui(&mut self) {
        self.ui_catalog = ui::packs::discover(&self.ui_folder);
        self.ui_scanned = Instant::now();
    }

    fn import_ui(&mut self, paths: Option<Vec<PathBuf>>, context: &egui::Context) {
        if self.importing_ui {
            return;
        }
        self.importing_ui = true;
        self.ui_error = None;
        let sender = self.sender.clone();
        let directory = self.ui_folder.clone();
        let context = context.clone();
        thread::spawn(move || {
            let paths = paths.unwrap_or_else(|| {
                rfd::FileDialog::new()
                    .set_title("Import UI config (JSON)")
                    .add_filter("UI config", &["json"])
                    .pick_files()
                    .unwrap_or_default()
            });
            let result = (|| {
                let mut ids = Vec::new();
                for path in paths {
                    ids.push(ui::packs::install(&path, &directory)?);
                }
                Ok(ids.join(", "))
            })();
            let _ = sender.send(Event::UiImport(result));
            context.request_repaint();
        });
    }

    fn switch_ui(&mut self, id: &str, context: &egui::Context) {
        if self.running.is_some()
            || self.importing_ui
            || self
                .login_task
                .as_ref()
                .is_some_and(|task| !task.is_finished())
        {
            return;
        }
        self.scan_ui();
        if !self.ui_catalog.packs.iter().any(|pack| pack.id == id) {
            self.ui_error = Some("That UI config is no longer available.".into());
            return;
        }
        self.ui_preferences.ui = id.to_owned();
        if let Ok(mut restart) = self.restart_request.lock() {
            *restart = true;
        }
        context.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn refresh_history(&mut self, context: &egui::Context, append: bool) {
        let append =
            append && self.history_filter == (self.show_archived, self.search_query.clone());
        if append && (self.history_loading || self.history_cursor.is_none()) {
            return;
        }
        let Some(runtime) = &self.runtime else {
            return;
        };
        self.history_generation += 1;
        let generation = self.history_generation;
        self.history_filter = (self.show_archived, self.search_query.clone());
        if !append {
            self.history.clear();
            self.history_cursor = None;
        }
        self.history_loading = true;
        self.history_error = None;
        let cursor = if append {
            self.history_cursor.clone()
        } else {
            None
        };
        let archived = self.show_archived;
        let search = self.search_query.clone();
        let executable = self.session.codex_path.clone();
        let sender = self.sender.clone();
        let context = context.clone();
        runtime.spawn(async move {
            let result = history::list(&executable, cursor, archived, &search).await;
            let _ = sender.send(Event::History(generation, append, result));
            context.request_repaint();
        });
    }

    fn open_history(&mut self, context: &egui::Context, thread: String, older: bool) {
        if self.running.is_some() || self.history_reading.is_some() {
            return;
        }
        let Some(runtime) = &self.runtime else {
            return;
        };
        let cursor = if older {
            self.conversation_cursors.get(&thread).cloned().flatten()
        } else {
            None
        };
        if older && cursor.is_none() {
            return;
        }
        self.history_reading = Some(thread.clone());
        self.history_error = None;
        self.status = "Loading saved Codex conversation...".into();
        let origin = self.session.active_chat;
        let archived = self.show_archived;
        let executable = self.session.codex_path.clone();
        let sender = self.sender.clone();
        let context = context.clone();
        runtime.spawn(async move {
            let result = history::read(&executable, &thread, cursor, archived).await;
            let _ = sender.send(Event::Conversation(thread, origin, older, result));
            context.request_repaint();
        });
    }

    fn send_prompt(&mut self, context: &egui::Context) {
        if self.startup.is_active()
            || self.running.is_some()
            || self.runtime.is_none()
            || self.picking_files
            || self.history_reading.is_some()
        {
            return;
        }
        self.attachment_error = None;

        let id = match self.session.prepare_run() {
            Ok(id) => id,
            Err(error) => {
                self.attachment_error = Some(error.clone());
                self.status = error;
                return;
            }
        };

        let Some(chat) = self.session.active() else {
            return;
        };

        let mut request = ChatRequest::from_chat(chat, self.effective_model().to_owned());
        request.effort = self.selected_effort();

        if let Some(runtime) = &self.runtime {
            self.running = Some(Running::start(
                runtime,
                id,
                request,
                self.session.codex_path.clone(),
                self.run_sender.clone(),
                context.clone(),
            ));
            self.status = "Generating...".into();
        }
    }

    fn stop_response(&mut self) {
        self.approval = None;
        if let Some(running) = self.running.take() {
            running.cancel();
            self.session
                .finish_run(running.id, MessageStatus::Cancelled, None);
            self.status = "Cancelled. Partial response kept.".into();
        }
    }

    fn effective_model(&self) -> &str {
        if self.session.model == DEFAULT_MODEL {
            self.default_model.as_deref().unwrap_or(DEFAULT_MODEL)
        } else {
            &self.session.model
        }
    }

    fn selected_model_info(&self) -> Option<&ModelInfo> {
        self.model_details
            .iter()
            .find(|model| model.model == self.effective_model())
    }

    fn selected_effort(&self) -> Option<String> {
        self.selected_model_info()
            .and_then(|model| {
                model.effective_effort(
                    self.session
                        .model_efforts
                        .get(&model.model)
                        .map(String::as_str),
                )
            })
            .map(str::to_owned)
    }

    fn set_usage(&mut self, result: Result<UsageInfo, String>) {
        match result {
            Ok(info) => {
                self.usage = Some(info);
                self.usage_error = None;
                self.usage_updated = Some(Instant::now());
            }
            Err(error) => self.usage_error = Some(error),
        }
    }

    fn refresh_usage(&mut self, context: &egui::Context) {
        if self.refreshing_usage {
            return;
        }
        let Some(runtime) = &self.runtime else {
            return;
        };
        self.refreshing_usage = true;
        let executable = self.session.codex_path.clone();
        let sender = self.sender.clone();
        let context = context.clone();
        runtime.spawn(async move {
            let result = codex::usage_limits(&executable).await;
            let _ = sender.send(Event::Usage(result));
            context.request_repaint();
        });
    }

    fn open_usage(&mut self, context: &egui::Context) {
        self.usage_open = true;
        self.refresh_usage(context);
    }

    fn refresh_models(&mut self, context: &egui::Context) {
        if self.refreshing {
            return;
        }

        self.refreshing = true;
        self.session.codex_path = codex::resolve_executable(&self.session.codex_path);
        self.status = "Connecting to Codex with your ChatGPT account...".into();

        let sender = self.sender.clone();
        let context = context.clone();
        let executable = self.session.codex_path.clone();

        if let Some(runtime) = &self.runtime {
            runtime.spawn(async move {
                let result = codex::connection_info(&executable).await;
                let _ = sender.send(Event::Models(result));
                context.request_repaint();
            });
        } else {
            self.refreshing = false;
        }
    }

    fn start_login(&mut self, context: &egui::Context) {
        let Some(runtime) = &self.runtime else {
            return;
        };
        if self
            .login_task
            .as_ref()
            .is_some_and(|task| !task.is_finished())
        {
            return;
        }
        let (sender, receiver) = tokio::sync::mpsc::channel(4);
        let context = context.clone();
        self.login_receiver = Some(receiver);
        self.session.codex_path = codex::resolve_executable(&self.session.codex_path);
        let executable = self.session.codex_path.clone();
        self.login_text = "Starting ChatGPT sign-in...".into();
        self.login_challenge = None;
        self.login_task = Some(runtime.spawn(async move {
            let result = codex::login(&executable, sender.clone()).await;
            let event = match result {
                Ok(()) => LoginEvent::Complete,
                Err(error) => LoginEvent::Failed(error),
            };
            let _ = sender.send(event).await;
            context.request_repaint();
        }));
    }

    fn pick_workspace(&mut self, context: &egui::Context) {
        if self.picking_workspace {
            return;
        }

        self.picking_workspace = true;

        let sender = self.sender.clone();
        let context = context.clone();

        thread::spawn(move || {
            let result = rfd::FileDialog::new()
                .set_title("Open workspace")
                .pick_folder()
                .map(std::fs::canonicalize)
                .transpose()
                .map_err(|error| format!("Could not open workspace: {error}"));

            let _ = sender.send(Event::Workspace(result));
            context.request_repaint();
        });
    }

    fn pick_files(&mut self, context: &egui::Context) {
        if self.picking_files {
            return;
        }
        if self.session.active().is_none() {
            self.session.new_chat();
        }
        let Some(chat) = self.session.active() else {
            return;
        };
        let chat_id = chat.id;
        self.picking_files = true;
        self.attachment_error = None;
        let sender = self.sender.clone();
        let context = context.clone();
        thread::spawn(move || {
            let paths = rfd::FileDialog::new()
                .set_title("Attach files")
                .pick_files()
                .unwrap_or_default();
            let result = read_attachments(paths);
            let _ = sender.send(Event::Attachments(chat_id, result));
            context.request_repaint();
        });
    }

    fn pick_codex_executable(&mut self, context: &egui::Context) {
        let sender = self.sender.clone();
        let context = context.clone();
        thread::spawn(move || {
            let dialog = rfd::FileDialog::new().set_title("Select Codex CLI");
            #[cfg(windows)]
            let dialog = dialog.add_filter("Executable", &["exe"]);
            let path = dialog.pick_file();
            let _ = sender.send(Event::CodexExecutable(path));
            context.request_repaint();
        });
    }

    fn receive_events(&mut self, context: &egui::Context) {
        let mut refresh_account = false;
        if let Some(receiver) = &mut self.login_receiver {
            while let Ok(event) = receiver.try_recv() {
                match event {
                    LoginEvent::Challenge(challenge) => {
                        self.login_text = "Open the sign-in page and enter this code.".into();
                        self.login_challenge = Some(challenge);
                    }
                    LoginEvent::Complete => {
                        self.login_challenge = None;
                        self.login_text = "ChatGPT sign-in complete.".into();
                        refresh_account = true;
                    }
                    LoginEvent::Failed(error) => {
                        self.login_challenge = None;
                        self.login_text = error;
                    }
                }
            }
        }
        if refresh_account {
            self.refresh_models(context);
        }

        for _ in 0..256 {
            let Ok(event) = self.run_receiver.try_recv() else {
                break;
            };

            let active = self.running.as_ref().map(|running| running.id);
            let id = event.run;

            if active == Some(id) {
                match event.update {
                    Update::Approval(approval) => {
                        self.approval = Some(approval);
                        self.status = "Waiting for your approval.".into();
                        continue;
                    }
                    Update::Status(text) => {
                        self.status = text;
                        continue;
                    }
                    _ => {}
                }
            }

            if run::apply_event(&mut self.session, active, event) {
                self.running = None;
                self.approval = None;
                self.status = self
                    .session
                    .chats
                    .iter()
                    .find(|chat| chat.id == id.chat)
                    .and_then(|chat| {
                        chat.messages
                            .iter()
                            .find(|message| message.id == id.message)
                    })
                    .and_then(|message| message.error.clone())
                    .unwrap_or_else(|| "Response complete.".into());
                if self.usage_open {
                    self.refresh_usage(context);
                }
                self.refresh_history(context, false);
            }
        }

        while let Ok(event) = self.receiver.try_recv() {
            match event {
                Event::UiImport(result) => {
                    self.importing_ui = false;
                    self.settings_open = true;
                    self.settings_page = ui::SettingsPage::Appearance;
                    match result {
                        Ok(id) => {
                            self.scan_ui();
                            self.status =
                                format!("Imported UI '{id}'. Select it from the UI menu.");
                        }
                        Err(error) => self.ui_error = Some(error),
                    }
                }
                Event::History(generation, append, result) => {
                    if generation != self.history_generation {
                        continue;
                    }
                    self.history_loading = false;
                    match result {
                        Ok(page) => {
                            if !append {
                                self.history.clear();
                            }
                            for entry in page.data {
                                self.history.retain(|old| old.id != entry.id);
                                self.history.push(entry);
                            }
                            self.history_cursor = page.next_cursor;
                            self.history_error = None;
                        }
                        Err(error) => self.history_error = Some(error),
                    }
                }
                Event::Conversation(thread, origin, older, result) => {
                    self.history_reading = None;
                    match result {
                        Ok(page) => {
                            self.conversation_cursors
                                .insert(thread, page.next_cursor.clone());
                            let id = history::import(&mut self.session, page, older);
                            if self.session.active_chat == origin {
                                self.session.active_chat = Some(id);
                            }
                            self.status = "Loaded saved Codex conversation.".into();
                        }
                        Err(error) => self.history_error = Some(error),
                    }
                }
                Event::CodexExecutable(Some(path)) => {
                    self.session.codex_path = codex::resolve_executable(&path.to_string_lossy());
                    self.refresh_models(context);
                }
                Event::CodexExecutable(None) => {}
                Event::Usage(result) => {
                    self.refreshing_usage = false;
                    self.set_usage(result);
                }
                Event::Attachments(chat_id, result) => {
                    self.picking_files = false;
                    match result {
                        Ok(attachments) => {
                            if let Some(chat) = self
                                .session
                                .chats
                                .iter_mut()
                                .find(|chat| chat.id == chat_id)
                            {
                                let new_count = attachments
                                    .iter()
                                    .filter(|file| {
                                        !chat
                                            .attachments
                                            .iter()
                                            .any(|existing| existing.path == file.path)
                                    })
                                    .count();
                                if chat.attachments.len() + new_count > MAX_ATTACHMENTS {
                                    self.attachment_error = Some(format!(
                                        "Attach at most {MAX_ATTACHMENTS} files per message."
                                    ));
                                } else {
                                    for attachment in attachments {
                                        chat.attachments
                                            .retain(|existing| existing.path != attachment.path);
                                        chat.attachments.push(attachment);
                                    }
                                }
                            }
                        }
                        Err(error) => self.attachment_error = Some(error),
                    }
                }
                Event::Models(result) => {
                    self.refreshing = false;

                    match result {
                        Ok(info) => {
                            self.account = info.account;
                            self.signed_in = info.signed_in;
                            self.default_model = info.default_model.clone();
                            self.model_details = info.model_details;
                            if !info.models.contains(&self.session.model) {
                                self.session.model =
                                    info.default_model.unwrap_or_else(|| DEFAULT_MODEL.into());
                            }
                            self.status = if self.signed_in {
                                format!("Codex connected · {}", self.account)
                            } else {
                                "Codex is available. Sign in with ChatGPT in Settings.".into()
                            };
                            self.models = info.models;
                            self.set_usage(info.usage);
                            self.refresh_history(context, false);
                        }
                        Err(error) => {
                            self.signed_in = false;
                            self.account = "Not connected".into();
                            self.models.clear();
                            self.status = error;
                        }
                    }
                }
                Event::Workspace(result) => {
                    self.picking_workspace = false;

                    match result {
                        Ok(Some(path)) => {
                            self.status = format!("Opened {}", path.display());
                            self.session.workspace = Some(path);
                            self.session.new_chat();
                            self.show_archived = false;
                        }
                        Ok(None) => {}
                        Err(error) => self.status = error,
                    }
                }
            }
        }
    }
}

fn read_attachments(paths: Vec<PathBuf>) -> Result<Vec<Attachment>, String> {
    if paths.len() > MAX_ATTACHMENTS {
        return Err(format!(
            "Attach at most {MAX_ATTACHMENTS} files per message."
        ));
    }
    paths.into_iter().map(Attachment::read).collect()
}

impl eframe::App for ForgeDesk {
    fn update(&mut self, context: &egui::Context, _frame: &mut eframe::Frame) {
        self.receive_events(context);
        let dropped = context.input(|input| {
            input
                .raw
                .dropped_files
                .iter()
                .filter_map(|file| file.path.clone())
                .collect::<Vec<_>>()
        });
        if !dropped.is_empty() {
            self.import_ui(Some(dropped), context);
        }
        if self.ui_scanned.elapsed() > Duration::from_secs(3) {
            self.scan_ui();
        }

        if context.input(|input| input.key_pressed(egui::Key::Escape)) {
            if self.settings_open {
                self.settings_open = false;
            } else if self.usage_open {
                self.usage_open = false;
            } else {
                self.stop_response();
            }
        }

        if self.running.is_some()
            || self
                .login_task
                .as_ref()
                .is_some_and(|task| !task.is_finished())
        {
            context.request_repaint_after(Duration::from_millis(33));
        }

        if !self.startup.is_active()
            && context.input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::N))
        {
            self.session.new_chat();
            self.show_archived = false;
        }

        if !self.startup.is_active()
            && context
                .input_mut(|input| input.consume_key(egui::Modifiers::COMMAND, egui::Key::Comma))
        {
            self.settings_open = true;
        }

        self.draw_ui(context);
        #[cfg(debug_assertions)]
        self.verify_ui_restart(context);
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        if self.capture_path.is_some() {
            return;
        }
        eframe::set_value(storage, STORAGE_KEY, &self.session);
        self.ui_preferences.sidebar_visible = self.sidebar_visible;
        self.ui_preferences.startup = Some(self.startup.preferences());
        self.ui_preferences
            .sidebars
            .insert(self.active_ui.id.clone(), self.sidebar_visible);
        eframe::set_value(storage, ui::PREFERENCES_KEY, &self.ui_preferences);
    }

    fn auto_save_interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}

impl Drop for ForgeDesk {
    fn drop(&mut self) {
        self.stop_response();
        if let Some(task) = self.login_task.take() {
            task.abort();
        }

        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_background();
        }
    }
}
