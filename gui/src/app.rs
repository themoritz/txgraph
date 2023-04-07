use std::collections::HashMap;

use egui::{CursorIcon, Frame, Sense, TextEdit};

use crate::{
    bitcoin::{BitcoinData, HttpClient, Transaction, Txid},
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

pub struct App {
    store: AppStore,
    bitcoin: HttpClient,
    transactions: HashMap<Txid, Transaction>,
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
            bitcoin: HttpClient::new(),
            transactions: HashMap::default(),
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

            let graph = to_drawable(&self.transactions);

            let toggle_tx = |txid: Txid| {
                if self.transactions.contains_key(&txid) {
                    self.transactions.remove(&txid);
                } else {
                    let r = self.bitcoin.get_transaction(txid);
                    println!("{:#?}", r);
                    self.transactions.insert(txid, r);
                }
            };

            graph.draw(ui, &self.transform, toggle_tx);
        });

        egui::Window::new("Controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Load Tx");
                ui.add(TextEdit::singleline(&mut self.store.tx));
                if ui.button("Go").clicked() {
                    let txid = Txid::new(&self.store.tx);
                    let r = self.bitcoin.get_transaction(txid);
                    println!("{:#?}", r);
                    self.transactions.insert(txid, r);
                }
            });
        });
    }
}
