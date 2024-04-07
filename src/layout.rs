use egui::Grid;
use serde::{Deserialize, Serialize};

use crate::bitcoin::Sats;

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Layout {
    pub force_params: ForceParams,
    pub scale: Scale,
}

impl Layout {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.force_params.ui(ui);
        self.scale.ui(ui);
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ForceParams {
    pub scale: f32,
    pub dt: f32,
    pub cooloff: f32,
    pub y_compress: f32,
    pub tx_repulsion_dropoff: f32,
}

impl Default for ForceParams {
    fn default() -> Self {
        Self {
            scale: 80.0,
            dt: 0.08,
            cooloff: 0.85,
            y_compress: 2.0,
            tx_repulsion_dropoff: 1.2,
        }
    }
}

impl ForceParams {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Graph layout parameters:");

        Grid::new("Layout").num_columns(2).show(ui, |ui| {
            ui.label("Scale:");
            ui.add(egui::Slider::new(&mut self.scale, 5.0..=200.0));
            ui.end_row();

            ui.label("Y Compress:");
            ui.add(egui::Slider::new(&mut self.y_compress, 1.0..=5.0));
            ui.end_row();

            ui.label("Tx repulsion factor:");
            ui.add(egui::Slider::new(&mut self.tx_repulsion_dropoff, 0.5..=2.0));
            ui.end_row();

            ui.label("Speed:");
            ui.add(egui::Slider::new(&mut self.dt, 0.001..=0.2));
            ui.end_row();

            ui.label("Cooloff:");
            ui.add(egui::Slider::new(&mut self.cooloff, 0.5..=0.99));
            ui.end_row();
        });
    }
}

/// Fit `y = a x^b` through `(x1, y1)` and `(x2, y2)`.
#[derive(Serialize, Deserialize)]
pub struct Scale {
    x1: u64,
    y1: f64,
    x2: u64,
    y2: f64,
}

#[allow(clippy::inconsistent_digit_grouping)]
impl Default for Scale {
    fn default() -> Self {
        Self {
            x1: 1_000_000,
            y1: 30.0,
            x2: 100_000_00_000_000,
            y2: 500.0,
        }
    }
}

impl Scale {
    pub fn apply(&self, x: u64) -> f64 {
        let b = -(self.y2 / self.y1).ln() / ((self.x1 as f64).ln() - (self.x2 as f64).ln());
        let a = self.y1 / (self.x1 as f64).powf(b);

        (a * (x as f64).powf(b)).max(10.0)
    }

    #[allow(clippy::inconsistent_digit_grouping)]
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Display size of transactions:");

        Grid::new("Scale").num_columns(2).show(ui, |ui| {
            ui.label("From:")
                .on_hover_text("The size of the smallest transaction you want to investigate.");
            ui.add(
                egui::Slider::new(&mut self.x1, 10_000..=100_000_00_000_000)
                    .custom_formatter(|x, _| format!("{}", Sats(x as u64)))
                    .logarithmic(true)
                    .text("sats"),
            );
            ui.end_row();

            ui.label("Size:")
                .on_hover_text("What size should the smallest transaction be?");
            ui.add(egui::Slider::new(&mut self.y1, 30.0..=500.0).text("points"));
            ui.end_row();

            ui.label("To:")
                .on_hover_text("The size of the largest transaction you want to investigate.");
            ui.add(
                egui::Slider::new(&mut self.x2, 10_000..=100_000_00_000_000)
                    .custom_formatter(|x, _| format!("{}", Sats(x as u64)))
                    .logarithmic(true)
                    .text("sats"),
            );
            ui.end_row();

            ui.label("Size:")
                .on_hover_text("What size should the largest transaction be?");
            ui.add(egui::Slider::new(&mut self.y2, 30.0..=500.0).text("points"));
            ui.end_row();
        });
    }
}
