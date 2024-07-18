use std::sync::mpsc::{channel, Receiver, Sender};

use crate::client::Client;

pub struct Account {
    input_email: String,
    input_password: String,
    sender: Sender<Msg>,
    receiver: Receiver<Msg>,
}

impl Default for Account {
    fn default() -> Self {
        let (sender, receiver) = channel();

        Self {
            input_email: String::new(),
            input_password: String::new(),
            sender,
            receiver,
        }
    }
}

enum Msg {
    Clear,
}

impl Account {
    pub fn show_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();

        for msg in self.receiver.try_iter() {
            match msg {
                Msg::Clear => {
                    self.input_email.clear();
                    self.input_password.clear();
                }
            }
        }

        if let Some(user) = Client::user_data(&ctx) {
            ui.label(format!("Logged in as: {}", user.email));
            if ui.button("Log out").clicked() {
                Client::logout(&ctx);
            }
        } else {
            ui.label("Email");
            ui.text_edit_singleline(&mut self.input_email);
            ui.label("Password");
            ui.text_edit_singleline(&mut self.input_password);

            if ui.button("Log in").clicked() {
                let sender = self.sender.clone();
                Client::login(&ctx, &self.input_email, &self.input_password, move |res| {
                    if res.is_some() {
                        sender.send(Msg::Clear).unwrap();
                    }
                });
            }
        }
    }
}
