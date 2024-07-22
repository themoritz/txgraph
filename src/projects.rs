use std::sync::mpsc::{channel, Receiver, Sender};

use chrono::Local;
use egui::{Grid, Label, RichText, TextEdit};
use egui_extras::{Column, TableBuilder};
use serde::{Deserialize, Serialize};

use crate::{
    client::{Client, ProjectEntry},
    export,
    notifications::Notifications,
};

#[derive(Default, Deserialize, Serialize)]
pub struct ProjectsWindow {
    open: bool,
    #[serde(skip)]
    projects: Projects,
}

struct Projects {
    import_text: String,
    input_email: String,
    input_password: String,
    projects: Option<LoadedProjects>,
    sender: Sender<Msg>,
    receiver: Receiver<Msg>,
}

impl Default for Projects {
    fn default() -> Self {
        let (sender, receiver) = channel();

        Self {
            import_text: String::new(),
            input_email: String::new(),
            input_password: String::new(),
            projects: None,
            sender,
            receiver,
        }
    }
}

struct LoadedProjects {
    projects: Vec<ProjectEntry>,
    active_project: Option<ActiveProject>,
}

impl LoadedProjects {
    fn new(projects: Vec<ProjectEntry>) -> Self {
        Self {
            projects,
            active_project: None,
        }
    }
}

struct ActiveProject {
    project: export::Project,
    id: i32,
}

enum Msg {
    Clear,
    SetProjects(Vec<ProjectEntry>),
    LoadProject(ActiveProject),
}

impl ProjectsWindow {
    pub fn show_toggle(&mut self, ui: &mut egui::Ui) {
        if ui.selectable_label(self.open, "Projects").clicked() {
            self.open = !self.open;
        }
    }

    pub fn show_window(
        &mut self,
        ctx: &egui::Context,
        open_project: impl FnOnce(export::Project),
        export_project: impl FnOnce(),
    ) {
        egui::Window::new("Projects")
            .open(&mut self.open)
            .show(ctx, |ui| {
                self.projects.show_ui(ui, open_project, export_project)
            });
    }
}

impl Projects {
    fn show_ui(
        &mut self,
        ui: &mut egui::Ui,
        open_project: impl FnOnce(export::Project),
        export_project: impl FnOnce(),
    ) {
        let ctx = ui.ctx().clone();

        for msg in self.receiver.try_iter() {
            match msg {
                Msg::Clear => {
                    self.input_email.clear();
                    self.input_password.clear();
                    self.projects = None;
                }
                Msg::SetProjects(projects) => {
                    self.projects = Some(LoadedProjects::new(projects));
                }
                Msg::LoadProject(active_project) => {
                    if let Some(projects) = &mut self.projects {
                        projects.active_project = Some(active_project);
                    }
                }
            }
        }

        if let Some(user) = Client::user_data(&ctx) {
            ui.horizontal(|ui| {
                ui.label("Logged in as:");
                ui.label(RichText::new(user.email).underline().strong());
                if ui.button("Log out").clicked() {
                    let sender = self.sender.clone();
                    Client::logout(&ctx, move || {
                        sender.send(Msg::Clear).unwrap();
                    });
                }
            });

            ui.separator();

            if let Some(loaded_projects) = &mut self.projects {
                TableBuilder::new(ui)
                    .striped(true)
                    .resizable(false)
                    .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                    .column(Column::remainder().at_least(60.0).clip(true).resizable(false))
                    .column(Column::auto())
                    .column(Column::auto().at_least(10.0))
                    .sense(egui::Sense::click())
                    .header(20.0, |mut header| {
                        header.col(|ui| {
                            ui.strong("Project");
                        });
                        header.col(|ui| {
                            ui.strong("Created");
                        });
                        header.col(|_ui| {
                        });
                    })
                    .body(|mut body| {
                        for project in &loaded_projects.projects {
                            body.row(20.0, |mut row| {
                                row.set_selected(Some(project.id) == loaded_projects.active_project.as_ref().map(|p| p.id));

                                row.col(|ui| {
                                    ui.add(Label::new(project.name.clone()).selectable(false));
                                });
                                row.col(|ui| {
                                    ui.add(Label::new(
                                        project
                                            .created_at
                                            .with_timezone(&Local)
                                            .format("%Y-%m-%d %H:%M")
                                            .to_string(),
                                    ).selectable(false));
                                });
                                row.col(|ui| {
                                    if project.is_public {
                                        ui.add(Label::new("✔").selectable(false));
                                    } else {
                                        ui.add(Label::new("").selectable(false));
                                    }
                                });

                                let sender = self.sender.clone();
                                let ctx2 = ctx.clone();
                                let id = project.id;
                                if row.response().clicked() {
                                    Client::load_project(&ctx, project.id, move |response| {
                                        match export::Project::import_json(response.data) {
                                            Ok(project) => {
                                                sender.send(Msg::LoadProject(ActiveProject { project, id })).unwrap();
                                            }
                                            Err(e) => {
                                                Notifications::error(&ctx2, "Could not load project", Some(e));
                                            }
                                        }

                                    });
                                }
                            });
                        }
                    });

                ui.separator();
                ui.button("New Project");
            } else {
                let sender = self.sender.clone();
                Client::list_projects(&ctx, move |projects| {
                    sender.send(Msg::SetProjects(projects)).unwrap();
                });
            }
        } else {
            ui.label(RichText::new("Log in or sign up to manage your projects:").strong());

            Grid::new("Login").num_columns(2).show(ui, |ui| {
                ui.label("Email");
                ui.text_edit_singleline(&mut self.input_email);
                ui.end_row();

                ui.label("Password");
                ui.text_edit_singleline(&mut self.input_password);
                ui.end_row();

                ui.label(""); // empty cell
                ui.horizontal(|ui| {
                    if ui.button("Log in").clicked() {
                        let sender = self.sender.clone();
                        Client::login(&ctx, &self.input_email, &self.input_password, move |res| {
                            if res.is_some() {
                                sender.send(Msg::Clear).unwrap();
                            }
                        });
                    }
                    if ui.button("Sign up").clicked() {
                        let sender = self.sender.clone();
                        Client::signup(&ctx, &self.input_email, &self.input_password, move |res| {
                            if res.is_some() {
                                sender.send(Msg::Clear).unwrap();
                            }
                        });
                    }
                });
                ui.end_row();
            });
        }

        ui.separator();

        ui.strong("Current Project");

        ui.horizontal(|ui| {
            if ui.button("Export JSON to Clipboard").clicked() {
                export_project();
                ui.close_menu();
            }
            ui.menu_button("Import from JSON", |ui| {
                ui.add(TextEdit::singleline(&mut self.import_text).hint_text("Paste JSON..."));
                if ui.button("Go").clicked() {
                    match export::Project::import(&self.import_text) {
                        Ok(project) => {
                            open_project(project);
                            self.import_text = String::new();
                        }
                        Err(e) => Notifications::error(&ctx, "Could not import Json", Some(e)),
                    }
                    ui.close_menu();
                }
            });
        });

        if let Some(user) = Client::user_data(&ctx) {
            if let Some(loaded_projects) = &self.projects {
                if let Some(active_project) = &loaded_projects.active_project {
                    let project = loaded_projects.projects.iter().find(|p| p.id == active_project.id).unwrap();

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {

                        }
                        ui.weak("Last Saved: 2021-09-01 12:34");
                    });
                    ui.horizontal(|ui| {
                        ui.button("Rename Project");

                        let mut public = project.is_public;
                        if ui.checkbox(&mut public, "Public").clicked() {
                            let sender = self.sender.clone();
                            let ctx2 = ctx.clone();
                            Client::set_project_public(
                                &ctx,
                                project.id,
                                public,
                                move || {
                                    Client::list_projects(&ctx2, move |projects| {
                                        sender.send(Msg::SetProjects(projects)).unwrap();
                                    });
                                },
                            );
                        }

                        ui.button("Delete Project");
                    });
                } else {
                    if ui.button("Save as New Project").clicked() {}
                }
            }
        } else {
            ui.weak("Log in or sign up to save your project");
        }
    }
}
