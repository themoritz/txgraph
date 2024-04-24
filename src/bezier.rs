use egui::{Color32, Mesh, Pos2, Sense, Vec2};

use crate::{bitcoin::Txid, transform::Transform};

pub struct Cubic {
    p0: Pos2,
    p1: Pos2,
    p2: Pos2,
    p3: Pos2,
}

impl Cubic {
    pub fn sankey(from: Pos2, to: Pos2) -> Self {
        let mid = Vec2::new(0.0, (to - from).y / 2.0);
        Cubic {
            p0: from,
            p1: from + mid,
            p2: to - mid,
            p3: to,
        }
    }

    /// Use as `Cubic::move_to().eval(t).y`.
    pub fn move_to() -> Self {
        Cubic {
            p0: Pos2::new(0.0, 0.0),
            p1: Pos2::new(0.0, 0.0),
            p2: Pos2::new(0.0, 1.0),
            p3: Pos2::new(1.0, 1.0),
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
    pub from_width: f32,
    pub to: Pos2,
    pub to_width: f32,
}

impl Edge {
    pub fn draw(
        &self,
        ui: &egui::Ui,
        color: Color32,
        draw_arrow: bool,
        transform: &Transform,
        coin: &(Txid, usize),
    ) -> egui::Response {
        let left = Cubic::sankey(self.from, self.to);
        let right = Cubic::sankey(
            self.from + Vec2::new(self.from_width, 0.0),
            self.to + Vec2::new(self.to_width, 0.0),
        );

        let steps = 15;

        let mut lefts = Vec::with_capacity(steps + 1);
        let mut rights = Vec::with_capacity(steps + 1);

        for n in 0..=steps {
            let t = n as f32 / steps as f32;
            lefts.push(transform.pos_to_screen(left.eval(t)));
            rights.push(transform.pos_to_screen(right.eval(t)));
        }

        let pointer = ui.ctx().pointer_latest_pos();
        let mut hovering = false;
        if let Some(p) = pointer {
            for n in 1..=steps {
                // Assuming that top and bot have the same x coords.
                let lt = lefts[n - 1];
                let lb = lefts[n];
                if p.y >= lt.y && p.y <= lb.y {
                    let rt = rights[n - 1];
                    let rb = rights[n];
                    if (lb - lt).rot90().dot(p - lt) >= 0. && (rb - rt).rot90().dot(p - rt) <= 0. {
                        hovering = true;
                        break;
                    }
                }
            }
        }

        let arrow_color = color.gamma_multiply(0.25);

        let color = if hovering {
            color.gamma_multiply(0.5)
        } else {
            color.gamma_multiply(0.4)
        };

        if draw_arrow {

            let start = 0.3;
            let arrow_width = 2;
            let arrow_length = 4;
            let start_i = (start * steps as f32) as usize;
            let end_i = start_i + arrow_width;
            let m_start_i = start_i + arrow_length;
            let m_end_i = m_start_i + arrow_width;
            let m_start = lefts[m_start_i] + (rights[m_start_i] - lefts[m_start_i]) / 2.;
            let m_end = lefts[m_end_i] + (rights[m_end_i] - lefts[m_end_i]) / 2.;

            let mut mesh = Mesh::default();
            mesh.colored_vertex(lefts[0], color);
            mesh.colored_vertex(rights[0], color);
            for n in 1..=start_i {
                let i0 = (n as u32 - 1) * 2;
                mesh.colored_vertex(lefts[n], color);
                mesh.colored_vertex(rights[n], color);
                mesh.add_triangle(i0, i0 + 1, i0 + 2);
                mesh.add_triangle(i0 + 1, i0 + 2, i0 + 3);
            }

            mesh.colored_vertex(m_start, color);

            let i0 = mesh.vertices.len() as u32 - 1;
            mesh.add_triangle(i0, i0 - 1, i0 - 2);

            mesh.colored_vertex(m_end, color);
            let tip_i = mesh.vertices.len() as u32 - 1;

            mesh.colored_vertex(lefts[end_i], color);
            mesh.colored_vertex(rights[end_i], color);
            for n in end_i + 1..=m_end_i + 1 {
                let i0 = tip_i + (n - end_i) as u32 * 2;
                mesh.colored_vertex(lefts[n], color);
                mesh.colored_vertex(rights[n], color);
                mesh.add_triangle(i0, tip_i, i0 + 2);
                mesh.add_triangle(i0 - 1, tip_i, i0 + 1);
            }

            let last = mesh.vertices.len() as u32 - 1;
            mesh.add_triangle(tip_i, last, last - 1);

            for n in m_end_i + 2..=steps {
                let i0 = last + (n - m_end_i - 2) as u32 * 2;
                mesh.colored_vertex(lefts[n], color);
                mesh.colored_vertex(rights[n], color);
                mesh.add_triangle(i0, i0 + 1, i0 + 2);
                mesh.add_triangle(i0 - 1, i0, i0 + 1);
            }

            ui.painter().add(mesh);

            // Arrow
            let mut mesh = Mesh::default();
            mesh.colored_vertex(lefts[start_i], arrow_color);
            mesh.colored_vertex(lefts[end_i], arrow_color);
            mesh.colored_vertex(m_start, arrow_color);
            mesh.colored_vertex(m_end, arrow_color);
            mesh.colored_vertex(rights[start_i], arrow_color);
            mesh.colored_vertex(rights[end_i], arrow_color);
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(1, 2, 3);
            mesh.add_triangle(2, 3, 4);
            mesh.add_triangle(3, 4, 5);
            ui.painter().add(mesh);

        } else {

            let mut mesh = Mesh::default();
            mesh.colored_vertex(lefts[0], color);
            mesh.colored_vertex(rights[0], color);
            for n in 1..=steps {
                let i0 = (n as u32 - 1) * 2;
                mesh.colored_vertex(lefts[n], color);
                mesh.colored_vertex(rights[n], color);
                mesh.add_triangle(i0, i0 + 1, i0 + 2);
                mesh.add_triangle(i0 + 1, i0 + 2, i0 + 3);
            }
            ui.painter().add(mesh);

        }

        let id = ui.id().with("edge").with(coin);
        if let (Some(p), true) = (pointer, hovering) {
            ui.interact(
                egui::Rect::from_center_size(p, Vec2::splat(50.)),
                id,
                Sense::click(),
            )
        } else {
            // We need a form of Response with the same id even when we're not hovering so that
            // context menus don't disappear when leaving the edge.
            ui.interact(egui::Rect::ZERO, id, Sense::hover())
        }
    }
}
