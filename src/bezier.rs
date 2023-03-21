use egui::{Pos2, Vec2};

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
