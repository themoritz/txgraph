use std::collections::HashMap;

use egui::{CollapsingHeader, Color32, Grid, Painter, Stroke, TextEdit};
use serde::{Deserialize, Serialize};

use crate::{bitcoin::Txid, graph::Graph, style, transform::Transform};

#[derive(Default, Serialize, Deserialize)]
pub struct Annotations {
    tx_color: HashMap<Txid, [u8; 3]>,
    tx_label: HashMap<Txid, String>,
    tx_open: Vec<Txid>,
    coin_color: HashMap<(Txid, usize), [f32; 3]>,
    coin_label: HashMap<(Txid, usize), String>,
    coin_open: Vec<(Txid, usize)>,
}

impl Annotations {
    pub fn open_tx(&mut self, txid: Txid) {
        if !self.tx_open.contains(&txid) {
            self.tx_open.push(txid);
        }
    }

    pub fn tx_color(&self, txid: Txid) -> Color32 {
        self.tx_color
            .get(&txid)
            .map_or(style::BLUE, |c| Color32::from_rgb(c[0], c[1], c[2]))
    }

    pub fn tx_label(&self, txid: Txid) -> Option<String> {
        self.tx_label.get(&txid).map(|l| l.to_owned())
    }

    pub fn ui(
        &mut self,
        graph: &Graph,
        painter: &Painter,
        transform: &Transform,
        ui: &mut egui::Ui,
    ) {
        for (i, txid) in self.tx_open.clone().into_iter().enumerate() {
            let mut label = self
                .tx_label
                .get(&txid)
                .map_or(String::new(), |l| l.clone());
            let mut color = self.tx_color.get(&txid).map_or([0x1d, 0x9b, 0xf0], |c| *c);

            let response = CollapsingHeader::new(format!("{}..", &txid.hex_string()[0..32]))
                .default_open(true)
                .id_source(txid)
                .show(ui, |ui| {
                    Grid::new("Annotations").num_columns(2).show(ui, |ui| {
                        ui.label("Label:");
                        ui.add(TextEdit::singleline(&mut label).hint_text(txid.hex_string()));
                        ui.end_row();

                        ui.label("Color:");
                        ui.horizontal(|ui| {
                            ui.color_edit_button_srgb(&mut color);
                            if ui.button("x").clicked() {
                                color = [0x1d, 0x9b, 0xf0];
                            }
                        });
                        ui.end_row();

                        if ui.button("Close").clicked() {
                            self.tx_open.remove(i);
                        }
                    })
                });

            if response.header_response.hovered()
                || response.body_response.map_or(false, |r| r.hovered())
            {
                if let Some(pos) = graph.get_tx_pos(txid) {
                    painter.line_segment(
                        [
                            response.header_response.rect.right_center(),
                            transform.pos_to_screen(pos),
                        ],
                        Stroke::new(style::TX_STROKE_WIDTH, Color32::BLACK.gamma_multiply(0.5)),
                    );
                }
            }

            if label.is_empty() {
                self.tx_label.remove(&txid);
            } else {
                self.tx_label.insert(txid, label);
            }

            if color == [0x1d, 0x9b, 0xf0] {
                self.tx_color.remove(&txid);
            } else {
                self.tx_color.insert(txid, color);
            }
        }
    }
}
