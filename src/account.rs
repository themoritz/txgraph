use std::sync::mpsc::{Receiver, Sender};

use serde::Deserialize;

use crate::app::API_BASE;

pub struct Account {
    state: UserState,
    update_sender: Sender<Update>,
    update_receiver: Receiver<Update>,
    loading: bool,
    error: Option<String>,
}

enum Update {
    LoggedIn {
        email: String,
        session_id: String,
    },
    LoggedOut,
    Loading,
    LoadingDone,
    Error {
        err: String,
    },
}

impl Account {
    pub fn new() -> Self {
        let (update_sender, update_receiver) = std::sync::mpsc::channel();

        Self {
            state: UserState::Guest {
                login_email: Default::default(),
                login_password: Default::default(),
            },
            update_sender,
            update_receiver,
            loading: false,
            error: None,
        }
    }

    fn apply_updates(&mut self) {
        for update in self.update_receiver.try_iter() {
            match update {
                Update::LoggedIn { email, session_id } => {
                    self.state = UserState::User { email, session_id };
                    self.error = None;
                }
                Update::LoggedOut => {
                    self.state = UserState::Guest {
                        login_email: Default::default(),
                        login_password: Default::default(),
                    };
                    self.error = None;
                }
                Update::Loading => {
                    self.loading = true;
                }
                Update::LoadingDone => {
                    self.loading = false;
                }
                Update::Error { err } => {
                    self.error = Some(err);
                }
            }
        }
    }

    fn try_login(&self, email: &str, password: &str) {
        let body = serde_json::json!({
            "email": email,
            "password": password,
        });
        let request = ehttp::Request::json(format!("{API_BASE}/user/login"), &body).unwrap();
        self.update_sender.send(Update::Loading).unwrap();

        let sender = self.update_sender.clone();
        let email = email.to_owned();

        ehttp::fetch(request, move |response| {
            sender.send(Update::LoadingDone).unwrap();
            let error = |e: String| sender.send(Update::Error { err: e }).unwrap();
            match response {
                Ok(response) => {
                    if response.status == 200 {
                        if let Some(text) = response.text() {
                            #[derive(Deserialize)]
                            struct Response {
                                // user_id: usize,
                                session_id: String
                            }

                            match serde_json::from_str(text) {
                                Ok(Response { session_id }) => {
                                    sender.send(Update::LoggedIn { email, session_id }).unwrap();
                                }
                                Err(err) => {
                                    error(err.to_string());
                                }
                            }
                        } else {
                            error("No text body response".to_string());
                        }
                    } else {
                        error(response.text().map_or("".to_string(), |t| t.to_owned()));
                    }
                }
                Err(err) => {
                    error(err);
                }
            }
        });
    }

    fn try_logout(&self, session_id: &str) {
        let request = ehttp::Request {
            headers: ehttp::Headers::new(&[
                ("Session", session_id),
            ]),
            ..ehttp::Request::post(format!("{API_BASE}/user/logout"), vec![])
        };
        self.update_sender.send(Update::Loading).unwrap();

        let sender = self.update_sender.clone();

        ehttp::fetch(request, move |response| {
            sender.send(Update::LoadingDone).unwrap();
            let error = |e: String| sender.send(Update::Error { err: e }).unwrap();
            match response {
                Ok(response) => {
                    if response.status == 200 {
                        sender.send(Update::LoggedOut).unwrap();
                    } else {
                        error(response.text().map_or("".to_string(), |t| t.to_owned()));
                    }
                }
                Err(err) => {
                    error(err);
                }
            }
        });
    }

    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        self.apply_updates();

        match self.state {
            UserState::Guest {
                ref mut login_email,
                ref mut login_password
            } => {
                ui.label("Login");
                ui.text_edit_singleline(login_email);
                ui.label("Password");
                ui.text_edit_singleline(login_password);

                let email = login_email.clone();
                let password = login_password.clone();

                if ui.button("Log in").clicked() {
                    self.try_login(&email, &password);
                }
            }
            UserState::User { ref email, ref session_id } => {
                ui.label(format!("Logged in as: {}", email));
                if ui.button("Log out").clicked() {
                    self.try_logout(session_id);
                }
            }

        }

        if let Some(ref error) = self.error {
            ui.label(error);
        }

        if self.loading {
            ui.spinner();
        }
    }
}

enum UserState {
    Guest {
        login_email: String,
        login_password: String,
    },
    User {
        email: String,
        session_id: String,
    },
}
