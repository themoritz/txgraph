use eframe::epaint::PathShape;
use egui::{Pos2, Painter, Color32, Stroke, Vec2, Rect, Rounding};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct App {
    from: Pos2,
    to: Pos2,
    height: f32
}

impl Default for App {
    fn default() -> Self {
        Self {
            from: Pos2::new(50.0, 50.0),
            to: Pos2::new(250.0, 150.0),
            height: 50.0
        }
    }
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_visuals(egui::Visuals::light());

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {

            let size = Vec2::new(10.0, self.height);
            let (response, painter) = ui.allocate_painter(ui.available_size_before_wrap(), egui::Sense::hover());

            let from_rect = Rect::from_min_size(self.from - Vec2::new(10.0, 0.0), size);
            let from_response = ui.interact(from_rect, response.id.with(1), egui::Sense::drag());
            self.from += from_response.drag_delta();
            let color = ui.style().interact(&from_response).bg_fill;
            painter.rect(from_rect.translate(from_response.drag_delta()), Rounding::none(), color, Stroke::NONE);

            let to_rect = Rect::from_min_size(self.to, size);
            let to_response = ui.interact(to_rect, response.id.with(2), egui::Sense::drag());
            self.to += to_response.drag_delta();
            let color = ui.style().interact(&to_response).bg_fill;
            painter.rect(to_rect.translate(to_response.drag_delta()), Rounding::none(), color, Stroke::NONE);

            let edge = Edge { height: self.height, from: self.from, to: self.to };
            edge.ui_content(&painter);
        });

        egui::Window::new("Controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("Height");
                ui.add(egui::widgets::Slider::new(&mut self.height, 1.0..=100.0));
            });
        });
    }
}

struct Edge {
    height: f32,
    from: Pos2,
    to: Pos2
}

impl Edge {
    fn ui_content(self, painter: &Painter) {
        let mid = Vec2::new((self.to - self.from).x / 2.0, 0.0);
        let p1 = self.from + mid;
        let p2 = self.to - mid;
        let down = Vec2::new(0.0, self.height);

        let mut last = self.from;

        let steps = (((self.to.x - self.from.x).abs() + (self.to.y - self.from.y).abs()) / 4.0) as i32;

        for n in 1..=steps {
            let t = n as f32 / steps as f32;
            let c = 1.0 - t;
            let c2 = c * c;
            let t2 = t * t;
            let p = Pos2::new(
                c2 * c * self.from.x + 3.0 * c2 * t * p1.x + 3.0 * c * t2 * p2.x + t2 * t * self.to.x,
                c2 * c * self.from.y + 3.0 * c2 * t * p1.y + 3.0 * c * t2 * p2.y + t2 * t * self.to.y
            );
            painter.add(PathShape::convex_polygon(vec![
                last,
                p,
                p + down,
                last + down
            ], Color32::LIGHT_BLUE, Stroke::new(0.5, Color32::LIGHT_BLUE)));
            last = p;
        }
    }
}
