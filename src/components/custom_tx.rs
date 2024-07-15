use egui::{Button, Pos2, TextEdit, TextStyle, Vec2};
use serde::{Deserialize, Serialize};

use crate::bitcoin::Txid;

#[derive(Default, Serialize, Deserialize)]
pub struct CustomTx {
    tx: String,
}

impl CustomTx {
    pub fn ui(&mut self, ui: &mut egui::Ui, load_tx: impl Fn(Txid, Option<Pos2>)) {
        let glyph_width =
            ui.fonts(|f| f.glyph_width(&TextStyle::Body.resolve(ui.style()), '0'));
        ui.allocate_space(Vec2::new(glyph_width * 63.5, 0.0));

        ui.add(
            TextEdit::singleline(&mut self.tx)
                .hint_text("Enter Txid")
                .desired_width(f32::INFINITY),
        );

        ui.horizontal(|ui| match Txid::new(&self.tx) {
            Ok(txid) => {
                if ui.button("Go").clicked() {
                    load_tx(txid, None);
                    ui.close_menu();
                }
            }
            Err(e) => {
                ui.add_enabled(false, Button::new("Go"));
                ui.label(format!("Invalid Txid: {}", e));
            }
        });
    }
}
