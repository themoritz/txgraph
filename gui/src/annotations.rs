use std::collections::HashMap;

use egui::{Button, Color32, Grid, TextEdit};
use serde::{Deserialize, Serialize};

use crate::{bitcoin::Txid, style};

#[derive(Default, Serialize, Deserialize)]
pub struct Annotations {
    tx_color: HashMap<Txid, [u8; 3]>,
    tx_label: HashMap<Txid, String>,
    coin_color: HashMap<(Txid, usize), [u8; 3]>,
    coin_label: HashMap<(Txid, usize), String>,
}

impl Annotations {
    pub fn set_tx_color(&mut self, txid: Txid, color: Color32) {
        self.tx_color
            .insert(txid, [color.r(), color.g(), color.b()]);
    }

    pub fn tx_color(&self, txid: Txid) -> Color32 {
        self.tx_color
            .get(&txid)
            .map_or(style::BLUE, |c| Color32::from_rgb(c[0], c[1], c[2]))
    }

    pub fn tx_label(&self, txid: Txid) -> Option<String> {
        self.tx_label.get(&txid).map(|l| l.to_owned())
    }

    pub fn tx_menu(&mut self, txid: Txid, ui: &mut egui::Ui) {
        let mut label = self
            .tx_label
            .get(&txid)
            .map_or(String::new(), |l| l.clone());

        Grid::new("Annotations").num_columns(2).show(ui, |ui| {
            ui.label("Label:");
            ui.horizontal(|ui| {
                if ui
                    .add(
                        TextEdit::singleline(&mut label)
                            .hint_text(txid.hex_string())
                            .desired_width(300.0),
                    )
                    .lost_focus()
                {
                    ui.close_menu();
                };
                if ui.button("❌").clicked() {
                    label = String::new();
                    ui.close_menu();
                }
            });
            ui.end_row();

            ui.label("Color:");
            ui.horizontal(|ui| {
                let colors = [
                    Color32::RED,
                    Color32::GREEN,
                    Color32::GOLD,
                    Color32::BLACK,
                    Color32::from_rgb(0xd8, 0x10, 0xc6),
                ];
                for color in colors {
                    if ui.add(Button::new("  ").fill(color)).clicked() {
                        self.set_tx_color(txid, color);
                        ui.close_menu();
                    }
                }
                if ui.button("❌").clicked() {
                    self.tx_color.remove(&txid);
                    ui.close_menu();
                }
            });
            ui.end_row();
        });

        if ui.button("Reset").clicked() {
            label = String::new();
            self.tx_color.remove(&txid);
            ui.close_menu();
        }

        if label.is_empty() {
            self.tx_label.remove(&txid);
        } else {
            self.tx_label.insert(txid, label);
        }
    }
}
