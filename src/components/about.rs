use egui::Pos2;
use serde::{Deserialize, Serialize};

use crate::{
    bitcoin::Txid,
    widgets::{BulletPoint, UiExt},
};

#[derive(Deserialize, Serialize)]
pub struct About {
    open: bool,
}

impl Default for About {
    fn default() -> Self {
        About { open: true }
    }
}

impl About {
    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn show_toggle(&mut self, ui: &mut egui::Ui) {
        if ui.selectable_label(self.open, "About").clicked() {
            self.open = !self.open;
        }
    }

    pub fn show_window(
        &mut self,
        ctx: &egui::Context,
        load_tx: impl Fn(Txid, Option<Pos2>),
    ) -> Option<egui::Rect> {
        egui::Window::new("txgraph.info")
            .open(&mut self.open)
            .show(ctx, |ui| {
                ui.label("Visualizing Bitcoin's transaction graph.");

                ui.add_space(3.0);

                if ui.button("Load Example Transaction").clicked() {
                    load_tx(Txid::random_interesting(), None);
                }

                ui.add_space(3.0);

                ui.bold("Instructions:");
                let steps = [
                    "Load a custom transaction or pick one from the Hall of Fame via the 'Tx' menu.",
                    "Click on inputs / outputs to expand to the next transaction.",
                    "Drag/pinch screen to pan/zoom.",
                    "Drag transactions to adjust layout.",
                    "Right-click transactions or inputs/outputs.",
                ];

                for step in steps {
                    ui.add(BulletPoint::new(step));
                }

                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/themoritz/txgraph");
                    ui.label("⸱");
                    ui.hyperlink_to("Contact", "mailto:hello@txgraph.info");
                });
            })
            .map(|r| r.response.rect)
    }
}
