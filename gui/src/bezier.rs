use egui::{Color32, Mesh, Pos2, Vec2};

use crate::transform::Transform;

pub struct Cubic {
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    p3: Pos2,
}

impl Cubic {
    pub fn sankey(from: Pos2, to: Pos2) -> Self {
        let mid = Vec2::new((to - from).x / 2.0, 0.0);
        Cubic {
            p0: from,
            p1: from + mid,
            p2: to - mid,
            p3: to,
        }
    }

    pub fn eval(&self, t: f32) -> Pos2 {
        let c = 1.0 - t;
        let c2 = c * c;
        let t2 = t * t;
        Pos2::new(
            c2 * c * self.p0.x
                + 3.0 * c2 * t * self.p1.x
                + 3.0 * c * t2 * self.p2.x
                + t2 * t * self.p3.x,
            c2 * c * self.p0.y
                + 3.0 * c2 * t * self.p1.y
                + 3.0 * c * t2 * self.p2.y
                + t2 * t * self.p3.y,
        )
    }
}
pub struct Edge {
    pub from: Pos2,
    pub from_height: f32,
    pub to: Pos2,
    pub to_height: f32,
}

impl Edge {
    pub fn draw(&self, ui: &egui::Ui, transform: &Transform) {
        let color = Color32::LIGHT_BLUE.gamma_multiply(0.5);

        let top = Cubic::sankey(self.from, self.to);
        let bot = Cubic::sankey(
            self.from + Vec2::new(0.0, self.from_height),
            self.to + Vec2::new(0.0, self.to_height),
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

            let i0 = (n - 1) * 4;
            mesh.colored_vertex(transform.to_screen(last_top), color);
            mesh.colored_vertex(transform.to_screen(new_top), color);
            mesh.colored_vertex(transform.to_screen(new_bot), color);
            mesh.colored_vertex(transform.to_screen(last_bot), color);
            mesh.add_triangle(i0, i0 + 1, i0 + 2);
            mesh.add_triangle(i0, i0 + 2, i0 + 3);

            last_top = new_top;
            last_bot = new_bot;
        }

        ui.painter().add(mesh);
    }
}
