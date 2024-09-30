use egui::Grid;
use serde::{Deserialize, Serialize};

use crate::{bitcoin::Sats, export, widgets::UiExt};

#[derive(Deserialize, Serialize, Default)]
#[serde(default)]
pub struct Layout {
    pub force_params: ForceParams,
    pub scale: Scale,
    #[serde(default = "default_as_true")]
    pub show_arrows: bool,
}

fn default_as_true() -> bool {
    true
}

impl Layout {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        self.force_params.ui(ui);
        ui.separator();
        self.scale.ui(ui);
        ui.separator();
        ui.bold("Misc:");
        ui.checkbox(&mut self.show_arrows, "Show arrows on edges");
    }

    pub fn import(&mut self, layout: &export::Layout0) {
        self.force_params.scale = layout.scale;
        self.scale.x1 = layout.x1;
        self.scale.y1 = layout.y1;
        self.scale.x2 = layout.x2;
        self.scale.y2 = layout.y2;
    }

    pub fn export(&self) -> export::Layout0 {
        export::Layout0 {
            scale: self.force_params.scale,
            x1: self.scale.x1,
            y1: self.scale.y1,
            x2: self.scale.x2,
            y2: self.scale.y2
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct ForceParams {
    pub scale: u64,
    pub dt: f32,
    pub cooloff: f32,
    pub active: bool,
}

impl Default for ForceParams {
    fn default() -> Self {
        Self {
            scale: 50,
            dt: 0.08,
            cooloff: 0.85,
            active: true,
        }
    }
}

impl ForceParams {
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.bold("Graph layout params:");

        Grid::new("Layout").num_columns(2).show(ui, |ui| {
            ui.label("Layout Algorithm:");
            ui.checkbox(&mut self.active, "Active");
            ui.end_row();

            ui.label("Scale:");
            ui.add(egui::Slider::new(&mut self.scale, 5..=200));
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
    y1: u64,
    x2: u64,
    y2: u64,
}

#[allow(clippy::inconsistent_digit_grouping)]
impl Default for Scale {
    fn default() -> Self {
        Self {
            x1: 1_000_000,
            y1: 30,
            x2: 100_000_00_000_000,
            y2: 500,
        }
    }
}

impl Scale {
    pub fn apply(&self, x: u64) -> f64 {
        let b = -(self.y2 as f64 / self.y1 as f64).ln() / ((self.x1 as f64).ln() - (self.x2 as f64).ln());
        let a = self.y1 as f64 / (self.x1 as f64).powf(b);

        (a * (x as f64).powf(b)).max(10.0)
    }

    #[allow(clippy::inconsistent_digit_grouping)]
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.bold("Display size of transactions:");

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
            ui.add(egui::Slider::new(&mut self.y1, 30..=500).text("points"));
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
            ui.add(egui::Slider::new(&mut self.y2, 30..=500).text("points"));
            ui.end_row();
        });
    }
}
