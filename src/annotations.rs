use std::collections::HashMap;

use egui::{Button, Color32, Grid, TextEdit};
use serde::{Deserialize, Serialize};

use crate::{bitcoin::Txid, export};

#[derive(PartialEq, Eq, Debug, Default, Serialize, Deserialize, Clone)]
pub struct Annotations {
    tx_color: HashMap<Txid, [u8; 3]>,
    tx_label: HashMap<Txid, String>,
    coin_color: HashMap<(Txid, usize), [u8; 3]>,
    coin_label: HashMap<(Txid, usize), String>,
}

impl Annotations {
    const COLORS: [Color32; 7] = [
        Color32::RED,
        Color32::GREEN,
        Color32::GOLD,
        Color32::from_rgb(0, 255, 255),
        Color32::from_rgb(255, 0, 255),
        Color32::from_rgb(128, 0, 255),
        Color32::from_rgb(255, 128, 0),
    ];

    pub fn import(annotations: &export::Annotations0) -> Result<Self, String> {
        fn txids_from_strings<T: Clone>(
            map: &HashMap<String, T>,
        ) -> Result<HashMap<Txid, T>, String> {
            map.iter()
                .map(|(s, v)| {
                    let txid = Txid::new(s)?;
                    Ok((txid, v.clone()))
                })
                .collect::<Result<HashMap<_, _>, _>>()
        }

        fn txos_from_strings<T: Clone>(
            map: &HashMap<String, T>,
        ) -> Result<HashMap<(Txid, usize), T>, String> {
            map.iter()
                .map(|(s, v)| {
                    let parts: Vec<_> = s.split(':').collect();
                    if parts.len() != 2 {
                        return Err("Expected txo key separated by `:`".to_string());
                    }
                    let txid = Txid::new(parts[0])?;
                    let vout = parts[1].parse::<usize>().map_err(|e| e.to_string())?;
                    Ok(((txid, vout), v.clone()))
                })
                .collect::<Result<HashMap<_, _>, _>>()
        }

        let result = Self {
            tx_color: txids_from_strings(&annotations.tx_color)?,
            tx_label: txids_from_strings(&annotations.tx_label)?,
            coin_color: txos_from_strings(&annotations.coin_color)?,
            coin_label: txos_from_strings(&annotations.coin_label)?,
        };

        Ok(result)
    }

    pub fn export(&self) -> export::Annotations0 {
        fn txids_to_strings<T: Clone>(map: &HashMap<Txid, T>) -> HashMap<String, T> {
            map.iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect()
        }

        fn txos_to_strings<T: Clone>(map: &HashMap<(Txid, usize), T>) -> HashMap<String, T> {
            map.iter()
                .map(|((txid, vout), v)| (format!("{}:{}", txid, vout), v.clone()))
                .collect()
        }

        export::Annotations0 {
            tx_color: txids_to_strings(&self.tx_color),
            tx_label: txids_to_strings(&self.tx_label),
            coin_color: txos_to_strings(&self.coin_color),
            coin_label: txos_to_strings(&self.coin_label),
        }
    }

    pub fn set_tx_color(&mut self, txid: Txid, color: Color32) {
        self.tx_color
            .insert(txid, [color.r(), color.g(), color.b()]);
    }

    pub fn set_coin_color(&mut self, coin: (Txid, usize), color: Color32) {
        self.coin_color
            .insert(coin, [color.r(), color.g(), color.b()]);
    }

    pub fn tx_color(&self, txid: Txid) -> Option<Color32> {
        self.tx_color
            .get(&txid)
            .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
    }

    pub fn coin_color(&self, coin: (Txid, usize)) -> Option<Color32> {
        self.coin_color
            .get(&coin)
            .map(|c| Color32::from_rgb(c[0], c[1], c[2]))
    }

    #[allow(dead_code)]
    pub fn set_tx_label(&mut self, txid: Txid, label: String) {
        self.tx_label.insert(txid, label);
    }

    #[allow(dead_code)]
    pub fn set_coin_label(&mut self, coin: (Txid, usize), label: String) {
        self.coin_label.insert(coin, label);
    }

    pub fn tx_label(&self, txid: Txid) -> Option<String> {
        self.tx_label.get(&txid).map(|l| l.to_owned())
    }

    pub fn coin_label(&self, coin: (Txid, usize)) -> Option<String> {
        self.coin_label.get(&coin).map(|l| l.to_owned())
    }

    pub fn coin_menu(&mut self, coin: (Txid, usize), ui: &mut egui::Ui) {
        let mut label = self
            .coin_label
            .get(&coin)
            .map_or(String::new(), |l| l.clone());

        Grid::new("Annotations").num_columns(2).show(ui, |ui| {
            ui.label("Label:");
            ui.horizontal(|ui| {
                if ui
                    .add(TextEdit::singleline(&mut label).desired_width(300.0))
                    .lost_focus()
                {
                    ui.close_menu();
                };
                if ui.button("✖").clicked() {
                    label = String::new();
                    ui.close_menu();
                }
            });
            ui.end_row();

            ui.label("Color:");
            ui.horizontal(|ui| {
                for color in Self::COLORS {
                    if ui.add(Button::new("  ").fill(color)).clicked() {
                        self.set_coin_color(coin, color);
                        ui.close_menu();
                    }
                }
                if ui.button("✖").clicked() {
                    self.coin_color.remove(&coin);
                    ui.close_menu();
                }
            });
            ui.end_row();
        });

        if label.is_empty() {
            self.coin_label.remove(&coin);
        } else {
            self.coin_label.insert(coin, label);
        }
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
                if ui.button("✖").clicked() {
                    label = String::new();
                    ui.close_menu();
                }
            });
            ui.end_row();

            ui.label("Color:");
            ui.horizontal(|ui| {
                for color in Self::COLORS {
                    if ui.add(Button::new("  ").fill(color)).clicked() {
                        self.set_tx_color(txid, color);
                        ui.close_menu();
                    }
                }
                if ui.button("✖").clicked() {
                    self.tx_color.remove(&txid);
                    ui.close_menu();
                }
            });
            ui.end_row();
        });

        if label.is_empty() {
            self.tx_label.remove(&txid);
        } else {
            self.tx_label.insert(txid, label);
        }
    }
}
