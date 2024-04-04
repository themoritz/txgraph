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
    pub from_height: f32,
    pub to: Pos2,
    pub to_height: f32,
}

impl Edge {
    pub fn draw(&self, ui: &egui::Ui, color: Color32, transform: &Transform) -> EdgeResponse {
        let top = Cubic::sankey(self.from, self.to);
        let bot = Cubic::sankey(
            self.from + Vec2::new(0.0, self.from_height),
            self.to + Vec2::new(0.0, self.to_height),
        );

        let steps = 30;

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

        let arrow_color = color.gamma_multiply(0.25);

        let color = if hovering {
            color.gamma_multiply(0.5)
        } else {
            color.gamma_multiply(0.4)
        };

        let start = 0.3;
        let arrow_width = 2;
        let arrow_length = 3;
        let start_i = (start * steps as f32) as usize;
        let end_i = start_i + arrow_width;
        let m_start_i = start_i + arrow_length;
        let m_end_i = m_start_i + arrow_width;
        let m_start = tops[m_start_i] + (bots[m_start_i] - tops[m_start_i]) / 2.;
        let m_end = tops[m_end_i] + (bots[m_end_i] - tops[m_end_i]) / 2.;

        let mut mesh = Mesh::default();
        mesh.colored_vertex(tops[0], color);
        mesh.colored_vertex(bots[0], color);
        for n in 1..=start_i {
            let i0 = (n as u32 - 1) * 2;
            mesh.colored_vertex(tops[n], color);
            mesh.colored_vertex(bots[n], color);
            mesh.add_triangle(i0, i0 + 1, i0 + 2);
            mesh.add_triangle(i0 + 1, i0 + 2, i0 + 3);
        }

        mesh.colored_vertex(m_start, color);

        let i0 = mesh.vertices.len() as u32 - 1;
        mesh.add_triangle(i0, i0 - 1, i0 - 2);

        mesh.colored_vertex(m_end, color);
        let tip_i = mesh.vertices.len() as u32 - 1;

        mesh.colored_vertex(tops[end_i], color);
        mesh.colored_vertex(bots[end_i], color);
        for n in end_i+1..=m_end_i+1 {
            let i0 = tip_i + (n - end_i) as u32 * 2;
            mesh.colored_vertex(tops[n], color);
            mesh.colored_vertex(bots[n], color);
            mesh.add_triangle(i0, tip_i, i0 + 2);
            mesh.add_triangle(i0 - 1, tip_i, i0 + 1);
        }

        let last = mesh.vertices.len() as u32 - 1;
        mesh.add_triangle(tip_i, last, last - 1);

        for n in m_end_i+2..=steps {
            let i0 = last + (n - m_end_i - 2) as u32 * 2;
            mesh.colored_vertex(tops[n], color);
            mesh.colored_vertex(bots[n], color);
            mesh.add_triangle(i0, i0 + 1, i0 + 2);
            mesh.add_triangle(i0 - 1, i0, i0 + 1);
        }

        ui.painter().add(mesh);

        // Arrow
        let mut mesh = Mesh::default();
        mesh.colored_vertex(tops[start_i], arrow_color);
        mesh.colored_vertex(tops[end_i], arrow_color);
        mesh.colored_vertex(m_start, arrow_color);
        mesh.colored_vertex(m_end, arrow_color);
        mesh.colored_vertex(bots[start_i], arrow_color);
        mesh.colored_vertex(bots[end_i], arrow_color);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(1, 2, 3);
        mesh.add_triangle(2, 3, 4);
        mesh.add_triangle(3, 4, 5);
        ui.painter().add(mesh);

        EdgeResponse { hovering, clicked }
    }
}

pub struct EdgeResponse {
    pub hovering: bool,
    pub clicked: bool,
}
