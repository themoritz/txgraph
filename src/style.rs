use std::sync::Arc;

use egui::{Color32, Stroke, Response, FontId, Widget};
use wasm_bindgen::prelude::wasm_bindgen;

pub struct Style {
    pub tx_width: f32,
    pub tx_stroke_width: f32,
    pub tx_stroke_color: Color32,
    pub io_width: f32,
    pub io_highlight_color: Color32,
    pub io_bg: Color32,
    pub utxo_bg: Color32,
    pub btc: Color32,
    pub tx_bg: Color32,
    pub egui_style: Arc<egui::Style>
}

impl Style {
    pub fn light(egui_style: Arc<egui::Style>) -> Self {
        Self {
            tx_width: 39.0,
            tx_stroke_width: 1.0,
            tx_stroke_color: Color32::from_gray(128),
            io_width: 7.0,
            io_highlight_color: Color32::from_gray(32),
            io_bg: Color32::from_gray(248),
            utxo_bg: Color32::from_gray(128),
            btc: Color32::from_rgb(255, 153, 0),
            tx_bg: Color32::from_rgb(0x1d, 0x9b, 0xf0),
            egui_style
        }
    }

    pub fn dark(egui_style: Arc<egui::Style>) -> Self {
        Self {
            tx_stroke_color: Color32::from_gray(80),
            io_highlight_color: Color32::from_gray(160),
            io_bg: Color32::from_gray(64),
            utxo_bg: Color32::from_gray(128),
            btc: Color32::from_rgb(255, 153, 0),
            tx_bg: Color32::from_rgb(0x1d, 0x9b, 0xf0),
            ..Self::light(egui_style)
        }
    }

    pub fn black_text_color(&self) -> Color32 {
        self.egui_style.visuals.strong_text_color()
    }

    pub fn white_text_color(&self) -> Color32 {
        self.egui_style.visuals.text_color()
    }

    pub fn tx_stroke(&self) -> Stroke {
        Stroke::new(self.tx_stroke_width, self.tx_stroke_color)
    }

    pub fn utxo_fill(&self) -> Color32 {
        self.utxo_bg
    }

    pub fn fees_fill(&self) -> Color32 {
        self.tx_stroke_color
    }

    pub fn font_id(&self) -> FontId {
        FontId::monospace(10.0)
    }

    pub fn io_stroke(&self, response: &Response) -> Stroke {
        if response.is_pointer_button_down_on() || response.has_focus() {
            Stroke::new(self.tx_stroke_width * 2.0, self.io_highlight_color)
        } else if response.hovered() || response.highlighted() {
            Stroke::new(self.tx_stroke_width, self.io_highlight_color)
        } else {
            Stroke::new(self.tx_stroke_width, self.tx_stroke_color)
        }
    }
}

pub fn get(ui: &egui::Ui) -> Style {
    let egui_style = ui.style();
    if egui_style.visuals.dark_mode {
        Style::dark(egui_style.clone())
    } else {
        Style::light(egui_style.clone())
    }
}

#[derive(Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum Theme {
    Light,
    Dark,
    #[default]
    System
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = isDarkMode)]
    fn is_dark_mode() -> bool;
}

impl Theme {
    fn is_dark_mode(&self) -> bool {
        match self {
            Theme::Light => false,
            Theme::Dark => true,
            Theme::System => is_dark_mode()
        }
    }
}

pub struct ThemeSwitch <'a> {
    theme: &'a mut Theme
}

impl <'a> ThemeSwitch <'a> {
    pub fn new(theme: &'a mut Theme) -> Self {
        Self { theme }
    }
}

impl<'a> Widget for ThemeSwitch<'a> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let response = ui.menu_button("◑ Theme", |ui| {
            if ui.selectable_value(self.theme, Theme::System, "System").clicked() {
                ui.close_menu();
            }
            if ui.selectable_value(self.theme, Theme::Light, "Light").clicked() {
                ui.close_menu();
            }
            if ui.selectable_value(self.theme, Theme::Dark, "Dark").clicked() {
                ui.close_menu();
            }
        }).response;

        let old_dark_mode = ui.style().visuals.dark_mode;
        let dark_mode = self.theme.is_dark_mode();

        if old_dark_mode != dark_mode {
            ui.ctx().set_visuals(
                if dark_mode {
                    egui::Visuals::dark()
                } else {
                    egui::Visuals::light()
                }
            );
        }

        response
    }
}
