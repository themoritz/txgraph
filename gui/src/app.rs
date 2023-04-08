use std::{collections::HashMap, sync::Arc};

use egui::{mutex::Mutex, CursorIcon, Frame, Sense, TextEdit};

use crate::{
    bitcoin::{Transaction, Txid},
    graph::to_drawable,
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
    err: Option<String>,
    loading: bool,
}

pub struct App {
    store: AppStore,
    state: Arc<Mutex<AppState>>,
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

        App {
            store,
            state: Arc::new(Mutex::new(AppState {
                transactions: HashMap::default(),
                err: None,
                loading: false,
            })),
            transform: Transform::default(),
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.store);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let frame = Frame::canvas(&ctx.style()).inner_margin(0.0);

        let state = self.state.clone();

        let toggle_tx = |txid: Txid| {
            if state.lock().transactions.contains_key(&txid) {
                state.lock().transactions.remove(&txid);
            } else {
                let request = ehttp::Request::get(format!("http://127.0.0.1:1337/{}", txid));
                state.lock().loading = true;

                let state = state.clone();
                let ctx = ctx.clone();

                ehttp::fetch(request, move |response| {
                    state.lock().loading = false;
                    match response {
                        Ok(response) => {
                            if response.status == 200 {
                                if let Some(text) = response.text() {
                                    match serde_json::from_str(&text) {
                                        Ok(tx) => {
                                            println!("{:#?}", tx);
                                            state.lock().transactions.insert(txid, tx);
                                        }
                                        Err(err) => state.lock().err = Some(err.to_string()),
                                    }
                                } else {
                                    state.lock().err = Some("No text body response.".to_string());
                                }
                            } else {
                                state.lock().err = response.text().map(|t| t.to_owned());
                            }
                        }
                        Err(err) => {
                            state.lock().err = Some(err.to_string());
                        }
                    }
                    ctx.request_repaint();
                });
            }
        };

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

            let graph = to_drawable(&state.lock().transactions);

            graph.draw(ui, &self.transform, toggle_tx);
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
                let mut state = self.state.lock();
                if state.loading {
                    ui.spinner();
                }

                if let Some(err) = &state.err {
                    ui.label(format!("Error: {}", err));
                    if ui.button("Ok").clicked() {
                        state.err = None;
                    }
                }
            });
        });
    }
}
