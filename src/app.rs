use eframe::epaint::PathShape;
use egui::{Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2, TextEdit, Shape, Mesh};
use electrum_client::bitcoin::{Txid, hashes::hex::FromHex};

use crate::{bitcoin::{Bitcoin, Transaction}, bezier};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppStore {
    from: Pos2,
    to: Pos2,
    height: f32,
    tx: String
}

impl Default for AppStore {
    fn default() -> Self {
        Self {
            from: Pos2::new(50.0, 50.0),
            to: Pos2::new(250.0, 150.0),
            height: 50.0,
            tx: String::new()
        }
    }
}

pub struct App {
    store: AppStore,
    bitcoin: Bitcoin,
    transaction: Option<Transaction>,
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
            bitcoin: Bitcoin::new("raspibolt.local:50002").unwrap(),
            transaction: None
        }
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.store);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            let (response, painter) =
                ui.allocate_painter(ui.available_size_before_wrap(), egui::Sense::hover());

            if let Some(t) = &self.transaction {
                t.draw(&painter)
            }

            let from_rect = Rect::from_min_size(self.store.from - Vec2::new(10.0, 0.0), Vec2::new(10.0, self.store.height));
            let from_response = ui.interact(from_rect, response.id.with(1), egui::Sense::drag());
            self.store.from += from_response.drag_delta();
            let color = ui.style().interact(&from_response).bg_fill;
            painter.rect(
                from_rect.translate(from_response.drag_delta()),
                Rounding::none(),
                color,
                Stroke::NONE,
            );

            let to_rect = Rect::from_min_size(self.store.to, Vec2::new(10.0, self.store.height / 2.0));
            let to_response = ui.interact(to_rect, response.id.with(2), egui::Sense::drag());
            self.store.to += to_response.drag_delta();
            let color = ui.style().interact(&to_response).bg_fill;
            painter.rect(
                to_rect.translate(to_response.drag_delta()),
                Rounding::none(),
                color,
                Stroke::NONE,
            );

            let edge = Edge {
                height: self.store.height,
                from: self.store.from,
                to: self.store.to,
            };
            edge.ui_content(&painter);
        });

        egui::Window::new("Controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Height");
                ui.add(egui::widgets::Slider::new(&mut self.store.height, 1.0..=100.0));
            });
            ui.horizontal(|ui| {
                ui.label("Tx");
                ui.add(TextEdit::singleline(&mut self.store.tx));
                if ui.button("Go").clicked() {
                    let r = self.bitcoin.get_transaction(&Txid::from_hex(&self.store.tx).unwrap()).unwrap();
                    println!("{:#?}", r);
                    self.transaction = Some(r);
                }
            });
        });
    }
}

struct Edge {
    height: f32,
    from: Pos2,
    to: Pos2,
}

impl Edge {
    fn ui_content(self, painter: &Painter) {
        let top = bezier::Cubic::sankey(self.from, self.to);
        let bot = bezier::Cubic::sankey(
            self.from + Vec2::new(0.0, self.height),
            self.to + Vec2::new(0.0, self.height / 2.0)
        );

        let mut last_top = top.eval(0.0);
        let mut last_bot = bot.eval(0.0);

        let steps =
            (((self.to.x - self.from.x).abs() + (self.to.y - self.from.y).abs()) / 4.0) as u32;

        let mut mesh = Mesh::default();

        for n in 1..=steps {
            let t = n as f32 / steps as f32;
            let new_top = top.eval(t);
            let new_bot = bot.eval(t);

            mesh.colored_vertex(last_top.clone(), Color32::LIGHT_BLUE);
            mesh.colored_vertex(new_top.clone(), Color32::LIGHT_BLUE);
            mesh.colored_vertex(new_bot.clone(), Color32::LIGHT_BLUE);
            mesh.colored_vertex(last_bot.clone(), Color32::LIGHT_BLUE);
            let n0 = (n - 1) * 4;
            mesh.add_triangle(n0, n0 + 1, n0 + 2);
            mesh.add_triangle(n0, n0 + 2, n0 + 3);

            last_top = new_top;
            last_bot = new_bot;
        }

        painter.add(Shape::Mesh(mesh));
    }
}
