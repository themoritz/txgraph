use egui::{Pos2, Rect, Vec2};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Transform {
    z: f32,
    t_x: f32,
    t_y: f32,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            z: 1.0,
            t_x: 0.0,
            t_y: 0.0,
        }
    }
}

impl Transform {
    pub fn pos_to_screen(&self, pos: Pos2) -> Pos2 {
        Pos2::new(self.z * pos.x + self.t_x, self.z * pos.y + self.t_y)
    }

    pub fn pos_from_screen(&self, pos: Pos2) -> Pos2 {
        Pos2::new((pos.x - self.t_x) / self.z, (pos.y - self.t_y) / self.z)
    }

    pub fn vec_to_screen(&self, vec: Vec2) -> Vec2 {
        Vec2::new(vec.x * self.z, vec.y * self.z)
    }

    pub fn vec_from_screen(&self, vec: Vec2) -> Vec2 {
        Vec2::new(vec.x / self.z, vec.y / self.z)
    }

    pub fn rect_to_screen(&self, rect: Rect) -> Rect {
        Rect::from_min_max(self.pos_to_screen(rect.min), self.pos_to_screen(rect.max))
    }

    pub fn translate(&mut self, vec: Vec2) {
        self.t_x += vec.x;
        self.t_y += vec.y;
    }

    pub fn zoom(&mut self, zoom_delta: f32, origin: Pos2) {
        self.z *= zoom_delta;
        self.t_x = zoom_delta * self.t_x + origin.x - zoom_delta * origin.x;
        self.t_y = zoom_delta * self.t_y + origin.y - zoom_delta * origin.y;
    }

    pub fn reset_zoom(&mut self, pos: Pos2) {
        self.zoom(1.0 / self.z, pos);
    }

    /// Make `pos` map to the center of the screen. `screen_center` is in real coordinates.
    pub fn pan_to(&mut self, pos: Pos2, screen_center: Pos2) {
        self.translate(self.z * (screen_center - pos));
    }
}
