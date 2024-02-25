use egui::{Pos2, Vec2};

use crate::bezier;

#[derive(Default)]
pub struct Flight {
    active: bool,
    time: f32,
    from: Pos2,
    to: Pos2,
    last_pos: Pos2,
}

impl Flight {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self, from: Pos2, to: Pos2) {
        self.active = true;
        self.time = 0.0;
        self.from = from;
        self.to = to;
        self.last_pos = from;
    }

    /// Returns by how much the position has changed.
    pub fn update(&mut self) -> Vec2 {
        self.time += 0.05;
        if self.time > 1.0 {
            self.active = false;
        }
        let new_pos = self.pos();
        let delta = new_pos - self.last_pos;
        self.last_pos = new_pos;
        delta
    }

    /// Interpolate between `from` and `to` according to a cubic ease-in-out curve.
    fn pos(&self) -> Pos2 {
        let t = bezier::Cubic::move_to().eval(self.time).y;
        Pos2::new(
            self.from.x * (1.0 - t) + self.to.x * t,
            self.from.y * (1.0 - t) + self.to.y * t,
        )
    }

    pub fn interrupt(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
