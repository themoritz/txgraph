use std::sync::{
    mpsc::{channel, Receiver, Sender},
    Arc,
};

use chrono::{DateTime, Local, Utc};
use egui::{mutex::Mutex, Button, Context, Id, Label, TextEdit, Ui};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{app::Update, export, modal, notifications::NotifyExt, style, widgets::UiExt};

pub struct Workspaces {
    sender: Sender<Msg>,
    receiver: Arc<Mutex<Receiver<Msg>>>,
    update_sender: Sender<Update>,
    workspaces: Vec<Workspace>,
    current_workspace: Uuid,
    window_open: bool,
    input_new_name: Option<String>,
    input_import_json: Option<String>,
    input_rename: Option<String>,
    input_confirm_delete: bool,
    request_focus: bool,
}

/// This is a bit of a hack. Ideally, we'd like this to be part of [AppStore].
#[derive(Serialize, Deserialize)]
struct WorkspacesStore {
    current_workspace: Uuid,
    window_open: bool,
}

impl Workspaces {
    pub fn new(ctx: &Context, update_sender: Sender<Update>) -> Self {
        let (sender, receiver) = channel();
        ctx.data_mut(|d| d.insert_temp(Id::NULL, WorkspacesSender(sender.clone())));

        let workspace = Workspace::new("Unnamed".to_string());
        let current_workspace = workspace.id;

        Self {
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            update_sender,
            workspaces: vec![workspace],
            current_workspace,
            window_open: false,
            input_new_name: None,
            input_import_json: None,
            input_rename: None,
            input_confirm_delete: false,
            request_focus: false,
        }
    }

    pub fn save(&self, storage: &mut dyn eframe::Storage) {
        // We ideally don't want to break the data in this key, ever:
        eframe::set_value(storage, "workspaces", &self.workspaces);

        eframe::set_value(
            storage,
            "workspaces_store",
            &WorkspacesStore {
                current_workspace: self.current_workspace,
                window_open: self.window_open,
            },
        );
    }

    pub fn load(
        ctx: &Context,
        storage: &dyn eframe::Storage,
        update_sender: Sender<Update>,
    ) -> Self {
        let mut result = Self::new(ctx, update_sender);

        if let Some(workspaces) = eframe::get_value(storage, "workspaces") {
            result.workspaces = workspaces;
        }

        if let Some(workspaces_store) =
            eframe::get_value::<WorkspacesStore>(storage, "workspaces_store")
        {
            result.window_open = workspaces_store.window_open;
            result.current_workspace = workspaces_store.current_workspace;
        }

        if result.workspaces.is_empty() {
            result.workspaces = vec![Workspace::new("Unnamed".to_string())];
        }

        // Make sure `current_workspace` is actually part of the workspaces
        if result
            .workspaces
            .iter()
            .find(|p| p.id == result.current_workspace)
            .is_none()
        {
            result.current_workspace = result.workspaces.first().unwrap().id;
        }

        result
    }

    fn with_current(&mut self, f: impl FnOnce(&mut Workspace)) {
        let i = self
            .workspaces
            .iter()
            .position(|p| p.id == self.current_workspace)
            .unwrap();
        f(&mut self.workspaces[i]);
    }

    fn current(&self) -> &Workspace {
        &self
            .workspaces
            .iter()
            .find(|p| p.id == self.current_workspace)
            .unwrap()
    }

    pub fn current_data(&self) -> export::Workspace {
        self.current().data.clone()
    }

    fn apply_update(&mut self, msg: Msg) {
        match msg {
            Msg::New { name, data } => {
                let mut p = Workspace::new(name);
                if let Some(data) = data {
                    p.data = data;
                }
                let id = p.id;
                self.workspaces.push(p);
                self.apply_update(Msg::Select { id });
            }
            Msg::UpdateData { data } => {
                self.with_current(|p| p.data = data);
            }
            Msg::Select { id } => {
                self.current_workspace = id;
                self.update_sender
                    .send(Update::LoadWorkspace {
                        data: self.current_data(),
                    })
                    .unwrap();
            }
            Msg::Rename { name } => {
                self.with_current(|p| p.name = name);
            }
            // Msg::TogglePublic => {
            //     self.with_current(|p| p.is_public = !p.is_public);
            // }
            Msg::Delete => {
                self.workspaces.retain(|p| p.id != self.current_workspace);
                if let Some(p) = self.workspaces.first() {
                    self.apply_update(Msg::Select { id: p.id });
                } else {
                    self.apply_update(Msg::New {
                        name: "Unnamed".to_string(),
                        data: None,
                    });
                }
            }
        }
    }

    pub fn show_toggle(&mut self, ui: &mut egui::Ui) {
        if ui
            .selectable_label(self.window_open, "Workspaces")
            .clicked()
        {
            self.window_open = !self.window_open;
        }
    }

    pub fn show_window(&mut self, ctx: &Context) {
        let mut open = self.window_open;
        egui::Window::new("Workspaces")
            .open(&mut open)
            .show(ctx, |ui| self.show_ui(ui));
        self.window_open = open;
    }

    fn show_ui(&mut self, ui: &mut Ui) {
        let receiver = self.receiver.clone();
        for msg in receiver.lock().try_iter() {
            self.apply_update(msg);
        }

        TableBuilder::new(ui)
            .striped(true)
            .resizable(false)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(
                Column::remainder()
                    .at_least(60.0)
                    .clip(true)
                    .resizable(false),
            )
            .column(Column::auto())
            // .column(Column::auto().at_least(10.0))
            .sense(egui::Sense::click())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.bold("Name");
                });
                header.col(|ui| {
                    ui.bold("Created");
                });
                // header.col(|ui| {
                //     ui.bold("Public");
                // });
            })
            .body(|mut body| {
                for workspace in &self.workspaces {
                    body.row(20.0, |mut row| {
                        row.set_selected(workspace.id == self.current_workspace);

                        row.col(|ui| {
                            ui.add(Label::new(workspace.name.clone()).selectable(false));
                        });
                        row.col(|ui| {
                            ui.add(
                                Label::new(
                                    workspace
                                        .created_at
                                        .with_timezone(&Local)
                                        .format("%Y-%m-%d %H:%M")
                                        .to_string(),
                                )
                                .selectable(false),
                            );
                        });
                        // row.col(|ui| {
                        //     if workspace.is_public {
                        //         ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                        //             ui.add_space(3.0);
                        //             ui.add(Label::new("✔").selectable(false));
                        //         });
                        //     }
                        // });

                        if row.response().clicked() {
                            self.sender.send(Msg::Select { id: workspace.id }).unwrap();
                        }
                    });
                }
            });

        ui.add_space(3.0);

        ui.horizontal(|ui| {
            if ui.button("New Workspace").clicked() {
                self.input_new_name = Some("".to_string());
                self.request_focus = true;
            }
            if let Some(name) = &self.input_new_name {
                let old_name = name.clone();
                let mut new_name = name.clone();
                modal::show(&ui.ctx(), "New Workspace", |ui| {
                    let resp =
                        ui.add(TextEdit::singleline(&mut new_name).hint_text("Workspace name..."));
                    if self.request_focus {
                        resp.request_focus();
                        self.request_focus = false;
                    }

                    ui.add_space(3.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.input_new_name = None;
                        }
                        if ui
                            .add_enabled(!new_name.is_empty(), Button::new("Create"))
                            .clicked()
                        {
                            self.sender
                                .send(Msg::New {
                                    name: new_name.clone(),
                                    data: None,
                                })
                                .unwrap();
                            self.input_new_name = None;
                        }
                    });
                });
                if new_name != old_name {
                    self.input_new_name = Some(new_name);
                }
            }

            if ui.button("Import JSON").clicked() {
                self.input_import_json = Some("".to_string());
                self.request_focus = true;
            }
            if let Some(json) = &self.input_import_json {
                let old_json = json.clone();
                let mut new_json = json.clone();
                modal::show(&ui.ctx(), "Import Workspace", |ui| {
                    let theme = egui_extras::syntax_highlighting::CodeTheme::from_style(ui.style());

                    let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
                        let mut layout_job = egui_extras::syntax_highlighting::highlight(
                            ui.ctx(),
                            ui.style(),
                            &theme,
                            string,
                            "toml",
                        );
                        layout_job.wrap.max_width = wrap_width;
                        ui.fonts(|f| f.layout_job(layout_job))
                    };

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        let resp = ui.add(
                            egui::TextEdit::multiline(&mut new_json)
                                .font(style::get(ui).font_id())
                                .desired_rows(10)
                                .lock_focus(true)
                                .desired_width(f32::INFINITY)
                                .layouter(&mut layouter),
                        );
                        if self.request_focus {
                            resp.request_focus();
                            self.request_focus = false;
                        }
                    });

                    ui.add_space(3.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.input_import_json = None;
                        }
                        if ui
                            .add_enabled(!new_json.is_empty(), Button::new("Import"))
                            .clicked()
                        {
                            match serde_json::from_str(&new_json) {
                                Ok(data) => {
                                    self.sender
                                        .send(Msg::New {
                                            name: "JSON import".to_string(),
                                            data: Some(data),
                                        })
                                        .unwrap();
                                    self.input_import_json = None;
                                }
                                Err(e) => {
                                    ui.ctx().notify_error("Could not import JSON", Some(e));
                                }
                            }
                        }
                    });
                });
                if new_json != old_json {
                    self.input_import_json = Some(new_json);
                }
            }
        });

        ui.separator();
        ui.bold("Current Workspace:");

        ui.horizontal(|ui| {
            if ui.button("Rename").clicked() {
                self.input_rename = Some(self.current().name.to_string());
                self.request_focus = true;
            }
            if let Some(name) = &self.input_rename {
                let old_name = name.clone();
                let mut new_name = name.clone();
                modal::show(&ui.ctx(), "Rename Workspace", |ui| {
                    let resp =
                        ui.add(TextEdit::singleline(&mut new_name).hint_text("Workspace name..."));
                    if self.request_focus {
                        resp.request_focus();
                        self.request_focus = false;
                    }

                    ui.add_space(3.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.input_rename = None;
                        }
                        if ui
                            .add_enabled(!new_name.is_empty(), Button::new("Rename"))
                            .clicked()
                        {
                            self.sender
                                .send(Msg::Rename {
                                    name: new_name.clone(),
                                })
                                .unwrap();
                            self.input_rename = None;
                        }
                    });
                });
                if new_name != old_name {
                    self.input_rename = Some(new_name);
                }
            }

            if ui.button("Delete").clicked() {
                self.input_confirm_delete = true;
            }
            if self.input_confirm_delete {
                modal::show(&ui.ctx(), "Delete Workspace", |ui| {
                    ui.label("Are you sure you want to delete the current workspace?");

                    ui.add_space(3.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancel").clicked() {
                            self.input_confirm_delete = false;
                        }
                        if ui.button("Delete").clicked() {
                            self.sender.send(Msg::Delete).unwrap();
                            self.input_confirm_delete = false;
                        }
                    });
                });
            }

            // let mut is_public = self.current().is_public;
            // if ui.checkbox(&mut is_public, "Public").clicked() {
            //     self.sender.send(Msg::TogglePublic).unwrap();
            // }

            if ui.button("Export JSON").clicked() {
                let current = self.current();
                ui.ctx()
                    .copy_text(serde_json::to_string(&current.data).unwrap());
                ui.ctx().notify_success(format!(
                    "Exported workspace `{}` to clipboard.",
                    current.name
                ));
            }
        });

        ui.add_space(3.0);

        ui.horizontal_wrapped(|ui| {
            ui.bold("Note:");
            ui.label("This app is still in development and we don't guarantee data is stored in the Browser. If you want to save your workspaces, export them to JSON.");
        });
    }
}

enum Msg {
    New {
        name: String,
        data: Option<export::Workspace>,
    },
    UpdateData {
        data: export::Workspace,
    },
    Select {
        id: Uuid,
    },
    Rename {
        name: String,
    },
    // TogglePublic,
    Delete,
}

#[derive(Clone, Deserialize, Serialize)]
struct Workspace {
    is_owned: bool,
    is_public: bool,
    data: export::Workspace,
    id: Uuid,
    name: String,
    created_at: DateTime<Utc>,
}

impl Workspace {
    fn new(name: String) -> Self {
        Workspace {
            is_owned: true,
            is_public: false,
            data: export::Workspace::default(),
            id: Uuid::now_v7(),
            name,
            created_at: Utc::now(),
        }
    }
}

#[derive(Clone)]
struct WorkspacesSender(Sender<Msg>);

pub struct WorkspacesHandle;

impl WorkspacesHandle {
    pub fn update_workspace(ctx: &Context, data: export::Workspace) {
        if let Some(WorkspacesSender(sender)) = ctx.data(|d| d.get_temp(Id::NULL)) {
            sender.send(Msg::UpdateData { data }).unwrap();
        }
    }
}
