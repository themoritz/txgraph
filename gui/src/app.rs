use std::{
    collections::HashMap,
    sync::mpsc::{channel, Receiver, Sender, TryRecvError},
};

use egui::{CursorIcon, Frame, Sense, TextEdit};

use crate::{
    bitcoin::{Transaction, Txid},
    graph::{to_drawable, DrawableGraph},
    transform::Transform,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppStore {
    tx: String,
}

impl Default for AppStore {
    fn default() -> Self {
        Self { tx: String::new() }
    }
}

pub struct AppState {
    transactions: HashMap<Txid, Transaction>,
    graph: DrawableGraph,
    err: Option<String>,
    loading: bool,
}

pub enum Update {
    AddTx { txid: Txid, tx: Transaction },
    RemoveTx { txid: Txid },
    Loading,
    LoadingDone,
    Error { err: String },
}

impl AppState {
    pub fn apply_update(&mut self, update: Update) {
        match update {
            Update::AddTx { txid, tx } => {
                self.transactions.insert(txid, tx);
                self.graph = to_drawable(&self.transactions);
            }
            Update::RemoveTx { txid } => {
                self.transactions.remove(&txid);
                self.graph = to_drawable(&self.transactions);
            }
            Update::Loading => self.loading = true,
            Update::LoadingDone => self.loading = false,
            Update::Error { err } => self.err = Some(err),
        }
    }
}

pub struct App {
    store: AppStore,
    state: AppState,
    update_sender: Sender<Update>,
    update_receiver: Receiver<Update>,
    transform: Transform,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        // crate::bitcoin::get().unwrap();
        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let store = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            AppStore::default()
        };

        // let transactions = dummy_transactions();
        let transactions = HashMap::new();
        let graph = to_drawable(&transactions);
        let mut transform = Transform::default();
        transform.translate(cc.integration_info.window_info.size / 4.0);

        let (update_sender, update_receiver) = channel();

        App {
            store,
            state: AppState {
                transactions,
                graph,
                err: None,
                loading: false,
            },
            update_sender,
            update_receiver,
            transform,
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.store);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        loop {
            match self.update_receiver.try_recv() {
                Ok(update) => self.state.apply_update(update),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("channel disconnected!"),
            }
        }

        let sender = self.update_sender.clone();

        let toggle_tx = |txid: Txid| {
            if self.state.transactions.contains_key(&txid) {
                sender.send(Update::RemoveTx { txid }).unwrap();
            } else {
                let request = ehttp::Request::get(format!("http://127.0.0.1:1337/{}", txid));
                sender.send(Update::Loading).unwrap();

                let ctx = ctx.clone();
                let sender = sender.clone();

                ehttp::fetch(request, move |response| {
                    sender.send(Update::LoadingDone).unwrap();
                    match response {
                        Ok(response) => {
                            if response.status == 200 {
                                if let Some(text) = response.text() {
                                    match serde_json::from_str(&text) {
                                        Ok(tx) => {
                                            println!("{:#?}", tx);
                                            sender.send(Update::AddTx { txid, tx }).unwrap();
                                        }
                                        Err(err) => {
                                            sender
                                                .send(Update::Error {
                                                    err: err.to_string(),
                                                })
                                                .unwrap();
                                        }
                                    }
                                } else {
                                    sender
                                        .send(Update::Error {
                                            err: "No text body response".to_string(),
                                        })
                                        .unwrap();
                                }
                            } else {
                                sender
                                    .send(Update::Error {
                                        err: response
                                            .text()
                                            .map_or("".to_string(), |t| t.to_owned()),
                                    })
                                    .unwrap();
                            }
                        }
                        Err(err) => {
                            sender
                                .send(Update::Error {
                                    err: err.to_string(),
                                })
                                .unwrap();
                        }
                    }
                    ctx.request_repaint();
                });
            }
        };

        let frame = Frame::canvas(&ctx.style()).inner_margin(0.0);
        ctx.request_repaint();

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let mut response = ui.allocate_response(
                ui.available_size_before_wrap(),
                Sense::click_and_drag().union(Sense::hover()),
            );

            // Zoom
            if let Some(hover_pos) = response.hover_pos() {
                let zoom_delta = ui.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 {
                    self.transform.zoom(zoom_delta, hover_pos);
                }
            }

            // Drag
            if response.dragged_by(egui::PointerButton::Primary) {
                response = response.on_hover_cursor(CursorIcon::Grabbing);
                self.transform.translate(response.drag_delta());
            }

            self.state.graph.draw(ui, &self.transform, toggle_tx);
        });

        egui::Window::new("Controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Load Tx");
                ui.add(TextEdit::singleline(&mut self.store.tx));

                match Txid::new(&self.store.tx) {
                    Ok(txid) => {
                        if ui.button("Go").clicked() {
                            toggle_tx(txid);
                        }
                    }
                    Err(e) => {
                        ui.label(e);
                    }
                }
            });

            ui.horizontal(|ui| {
                if self.state.loading {
                    ui.spinner();
                }

                if let Some(err) = &self.state.err {
                    ui.label(format!("Error: {}", err));
                    if ui.button("Ok").clicked() {
                        self.state.err = None;
                    }
                }
            });
        });
    }
}
