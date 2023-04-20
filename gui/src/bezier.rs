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
    pub fn draw(&self, ui: &egui::Ui, transform: &Transform) -> EdgeResponse {
        let top = Cubic::sankey(self.from, self.to);
        let bot = Cubic::sankey(
            self.from + Vec2::new(0.0, self.from_height),
            self.to + Vec2::new(0.0, self.to_height),
        );

        let steps =
            (((self.to.x - self.from.x).abs() + (self.to.y - self.from.y).abs()) / 4.0) as usize;

        let mut tops = Vec::with_capacity(steps + 1);
        let mut bots = Vec::with_capacity(steps + 1);

        for n in 0..=steps {
            let t = n as f32 / steps as f32;
            tops.push(transform.pos_to_screen(top.eval(t)));
            bots.push(transform.pos_to_screen(bot.eval(t)));
        }

        let pointer = ui.ctx().pointer_latest_pos();
        let mut hovering = false;
        let mut clicked = false;
        if let Some(p) = pointer {
            for n in 1..=steps {
                // Assuming that top and bot have the same x coords.
                let tl = tops[n - 1];
                let tr = tops[n];
                if p.x >= tl.x && p.x <= tr.x {
                    let bl = bots[n - 1];
                    let br = bots[n];
                    // Equivalent to rotate(tr - tl).dot(p - tr)
                    if (tr.y - tl.y) * (p.x - tl.x) - (tr.x - tl.x) * (p.y - tl.y) <= 0.0
                        && (br.y - bl.y) * (p.x - bl.x) - (br.x - bl.x) * (p.y - bl.y) >= 0.0
                    {
                        hovering = true;
                        clicked = ui.input(|i| i.pointer.primary_clicked());
                        break;
                    }
                }
            }
        }

        let color = if hovering {
            Color32::GOLD.gamma_multiply(0.8)
        } else {
            Color32::GOLD.gamma_multiply(0.5)
        };

        let mut mesh = Mesh::default();
        for n in 1..=steps {
            let i0 = (n as u32 - 1) * 4;
            mesh.colored_vertex(tops[n - 1], color);
            mesh.colored_vertex(tops[n], color);
            mesh.colored_vertex(bots[n], color);
            mesh.colored_vertex(bots[n - 1], color);
            mesh.add_triangle(i0, i0 + 1, i0 + 2);
            mesh.add_triangle(i0, i0 + 2, i0 + 3);
        }

        ui.painter().add(mesh);

        EdgeResponse { hovering, clicked }
    }
}

pub struct EdgeResponse {
    pub hovering: bool,
    pub clicked: bool,
}
