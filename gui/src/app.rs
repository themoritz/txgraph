use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use egui::{CursorIcon, Frame, Pos2, Sense, TextEdit};

use crate::{
    bitcoin::{Transaction, Txid},
    graph::DrawableGraph,
    transform::Transform,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppStore {
    tx: String,
    layout_params: LayoutParams,
}

impl Default for AppStore {
    fn default() -> Self {
        Self {
            tx: String::new(),
            layout_params: LayoutParams::default(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LayoutParams {
    pub scale: f32,
    pub dt: f32,
    pub cooloff: f32,
    pub y_compress: f32,
    pub tx_repulsion_dropoff: f32,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            scale: 80.0,
            dt: 0.08,
            cooloff: 0.85,
            y_compress: 2.5,
            tx_repulsion_dropoff: 1.0,
        }
    }
}

pub struct AppState {
    graph: DrawableGraph,
    err: Option<String>,
    loading: bool,
}

pub enum Update {
    AddTx {
        txid: Txid,
        tx: Transaction,
        pos: Pos2,
    },
    RemoveTx {
        txid: Txid,
    },
    Loading,
    LoadingDone,
    Error {
        err: String,
    },
}

impl AppState {
    pub fn apply_update(&mut self, update: Update) {
        match update {
            Update::AddTx { txid, tx, pos } => {
                self.graph.add_tx(txid, tx, pos);
            }
            Update::RemoveTx { txid } => {
                self.graph.remove_tx(txid);
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

        let graph = DrawableGraph::empty();
        let mut transform = Transform::default();
        transform.translate(cc.integration_info.window_info.size / 4.0);

        let (update_sender, update_receiver) = channel();

        App {
            store,
            state: AppState {
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

        let remove_tx = |txid: Txid| {
            sender.send(Update::RemoveTx { txid }).unwrap();
        };

        let load_tx = |txid: Txid, pos: Pos2| {
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
                                        sender.send(Update::AddTx { txid, tx, pos }).unwrap();
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
                                    err: response.text().map_or("".to_string(), |t| t.to_owned()),
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

            self.state.graph.draw(
                ui,
                &self.transform,
                load_tx,
                remove_tx,
                &self.store.layout_params,
            );
        });

        egui::Window::new("Controls").show(ctx, |ui| {
            ui.collapsing("Layout", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Scale:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.scale,
                        5.0..=200.0,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Y Compress:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.y_compress,
                        1.0..=5.0,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Tx repulsion factor:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.tx_repulsion_dropoff,
                        0.5..=2.0,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("dt:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.dt,
                        0.001..=0.2,
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Cooloff:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.cooloff,
                        0.5..=0.99,
                    ));
                });
            });

            ui.horizontal(|ui| {
                ui.label("Load Tx");
                ui.add(TextEdit::singleline(&mut self.store.tx));

                match Txid::new(&self.store.tx) {
                    Ok(txid) => {
                        if ui.button("Go").clicked() {
                            load_tx(txid, Pos2::new(0.0, 0.0));
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
