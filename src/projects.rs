use std::sync::mpsc::{channel, Receiver, Sender};

use chrono::Local;
use egui::{Grid, RichText, TextEdit, Vec2};
use serde::{Deserialize, Serialize};

use crate::{
    client::{Client, ProjectEntry},
    export::Project,
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
    projects: Option<Vec<ProjectEntry>>,
    active_project: Option<i32>,
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
            active_project: None,
            sender,
            receiver,
        }
    }
}

enum Msg {
    Clear,
    SetProjects(Vec<ProjectEntry>),
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
        open_project: impl FnOnce(Project),
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
        open_project: impl FnOnce(Project),
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
                    self.projects = Some(projects);
                }
            }
        }

        ui.allocate_space(Vec2::new(350., 0.));

        if ui.button("Export to Clipboard").clicked() {
            export_project();
            ui.close_menu();
        }
        ui.menu_button("Import", |ui| {
            ui.add(TextEdit::singleline(&mut self.import_text).hint_text("Paste JSON..."));
            if ui.button("Go").clicked() {
                match Project::import(&self.import_text) {
                    Ok(project) => {
                        open_project(project);
                        self.import_text = String::new();
                    }
                    Err(e) => Notifications::error(&ctx, "Could not import Json", Some(e)),
                }
                ui.close_menu();
            }
        });

        if ui.button("Save as new project").clicked() {}

        ui.separator();

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

            if let Some(projects) = &self.projects {
                Grid::new("Projects")
                    .num_columns(3)
                    .striped(true)
                    .spacing(Vec2::new(10., 10.))
                    .show(ui, |ui| {
                        ui.label(RichText::new("Project").strong());
                        ui.label("Public");
                        ui.label("Created At");
                        ui.label("");
                        ui.end_row();

                        for project in projects {
                            if Some(project.id) == self.active_project {
                                let mut name = project.name.clone();
                                ui.add(
                                    TextEdit::singleline(&mut name)
                                        .desired_width(200.0)
                                        .hint_text("Name"),
                                );

                                let mut public = project.is_public;
                                if ui.checkbox(&mut public, "").clicked() {
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
                            } else {
                                ui.label(project.name.clone());

                                if project.is_public {
                                    ui.label("✔");
                                } else {
                                    ui.label("");
                                }
                            }

                            ui.label(
                                project
                                    .created_at
                                    .with_timezone(&Local)
                                    .format("%Y-%m-%d %H:%M")
                                    .to_string(),
                            );

                            ui.horizontal(|ui| {
                                if ui.button("Open").clicked() {
                                    self.active_project = Some(project.id);
                                }

                                if project.is_public {
                                    if ui.button("Link").clicked() {}
                                }
                            });

                            ui.end_row();
                        }
                    });
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
    }
}
