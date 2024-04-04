use std::f32::consts::TAU;

use egui::{Color32, Mesh, Pos2, Vec2};
use geo::{polygon, AffineOps, AffineTransform, BooleanOps, Coord, LineString, MultiPolygon, Polygon, TriangulateEarcut};

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

        let transparent_color = if hovering {
            color.gamma_multiply(0.5)
        } else {
            color.gamma_multiply(0.4)
        };

        let mut coords = Vec::with_capacity(steps * 2 + 1);

        for top in tops.iter() {
            coords.push(geo::Coord { x: top.x, y: top.y });
        }
        for bot in bots.iter().rev() {
            coords.push(geo::Coord { x: bot.x, y: bot.y });
        }

        let sankey = Polygon::new(LineString::new(coords), vec![]);

        let mid_start = tops[0] + (bots[0] - tops[0]) / 2.;
        let mid_end = tops[steps] + (bots[steps] - tops[steps]) / 2.;
        let mid_curve = Cubic::sankey(mid_start, mid_end);
        // let mid = mid_start + (mid_end - mid_start) / 2.;
        let mid = mid_curve.eval(0.2);
        let length = (mid_end - mid_start).length();
        let avg_height = (tops[0].y - bots[0].y).abs().max((tops[steps-1].y - bots[steps-1].y).abs());
        // let angle = (mid_end - mid_start).angle() * 360. / TAU;
        let angle = (mid_curve.eval(0.21) - mid_curve.eval(0.19)).angle() * 360. / TAU;
        let transform = AffineTransform::translate(mid.x, mid.y)
            .rotated(angle, Coord { x: 0.0, y: 0.0 })
            .scaled(1.0, avg_height.max(40.) / 100., Coord { x: 0.0, y: 0.0 });

        let arrow = polygon!(
            (x: 5., y: 0.),
            (x: -30., y: -100.),
            (x: -35., y: -100.),
            (x: 0., y: 0.),
            (x: -35., y: 100.),
            (x: -30., y: 100.),
        ).affine_transform(&transform);

        ui.painter().add(mesh_from_poly(&(sankey.difference(&arrow)), transparent_color));
        ui.painter().add(mesh_from_poly(&(sankey.intersection(&arrow)), color.gamma_multiply(0.15)));

        EdgeResponse { hovering, clicked }
    }
}

pub struct EdgeResponse {
    pub hovering: bool,
    pub clicked: bool,
}

fn mesh_from_poly(poly: &MultiPolygon<f32>, color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    let mut offset = 0;

    for p in poly.0.iter() {
        let triangulation = p.earcut_triangles_raw();

        for t in triangulation.triangle_indices.chunks_exact(3) {
            mesh.add_triangle(offset + t[0] as u32, offset + t[1] as u32, offset + t[2] as u32);
        }

        for v in triangulation.vertices.chunks_exact(2) {
            mesh.colored_vertex(Pos2::new(v[0], v[1]), color);
            offset += 1;
        }
    }

    mesh
}
