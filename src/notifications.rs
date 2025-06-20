use std::sync::mpsc::{Receiver, Sender, TryRecvError};

use egui::{
    lerp, Align2, Area, Color32, Context, Frame, Id, Order, Pos2, Rect, Response, RichText, Sense,
    Shape, Stroke, Ui, Vec2, WidgetText,
};

const FIRST_OFFSET: f32 = 0.0;
const PADDING: f32 = 15.0;
/// We can't know how tall the toats will be before they are rendered.
const INITIAL_FRAME_HEIGHT: f32 = 36.0;
const COOLOFF: f32 = 0.50;
const SPEED: f32 = 30.0;

#[derive(Clone, Debug)]
pub enum Kind {
    Error,
    Warn,
    Info,
    Success,
}

impl Kind {
    fn icon(&self) -> &str {
        match self {
            Kind::Error => "✘",
            Kind::Warn => "▲",
            Kind::Info => "ℹ",
            Kind::Success => "✔",
        }
    }

    fn color(&self) -> Color32 {
        match self {
            Kind::Error => Color32::from_rgb(255, 83, 83),
            Kind::Warn => Color32::from_rgb(255, 171, 83),
            Kind::Info => Color32::from_rgb(105, 135, 255),
            Kind::Success => Color32::from_rgb(47, 179, 57),
        }
    }

    fn icon_text(&self) -> WidgetText {
        WidgetText::from(self.icon()).color(self.color())
    }
}

#[derive(Clone, Debug)]
struct Toast {
    kind: Kind,
    message: String,
    detail: Option<String>,
    offset: f32,
    velocity: f32,
    ttl_sec: f32,
    initial_ttl_sec: f32,
    index: usize,
    /// We need to keep track of the last frame height to calculate the offset
    /// of the next toast.
    last_frame_height: f32,
}

impl Toast {
    fn new(kind: Kind, message: String, detail: Option<String>, ttl_sec: f32) -> Self {
        Self {
            kind,
            message,
            detail,
            offset: FIRST_OFFSET - INITIAL_FRAME_HEIGHT - PADDING,
            velocity: 0.0,
            ttl_sec,
            initial_ttl_sec: ttl_sec,
            index: 0,
            last_frame_height: INITIAL_FRAME_HEIGHT,
        }
    }

    /// Position the progress circle in the given [Rect].
    fn progress(&mut self, ui: &mut Ui, rect: Rect) -> Response {
        let response = ui
            .allocate_rect(rect, Sense::click())
            .on_hover_text("Close");

        if response.clicked() {
            self.ttl_sec = 0.0;
        }

        if ui.is_rect_visible(rect) {
            let stroke_width = 2.0;
            let bg_stroke = Stroke::new(stroke_width, ui.visuals().weak_text_color());
            let fg_stroke = Stroke::new(stroke_width, ui.visuals().strong_text_color());
            let n_points = 30;

            // Progress circle
            let progress = (self.ttl_sec / self.initial_ttl_sec) as f64;
            let radius = (rect.height() / 2.0) - 0.5 * stroke_width + 1.0;

            let start_angle = (0.75 - progress) * std::f64::consts::TAU;
            let end_angle = 0.75 * std::f64::consts::TAU;

            let points: Vec<Pos2> = (0..n_points)
                .map(|i| {
                    let angle = lerp(start_angle..=end_angle, i as f64 / n_points as f64);
                    let (sin, cos) = angle.sin_cos();
                    rect.center() + radius * Vec2::new(cos as f32, sin as f32)
                })
                .collect();

            ui.painter().circle(
                rect.center(),
                radius - stroke_width / 2.0,
                Color32::TRANSPARENT,
                bg_stroke,
            );
            ui.painter().add(Shape::line(points, fg_stroke));

            // Close cross
            let visuals = ui.style().interact(&response);
            let rect = rect.shrink(radius * 0.7).expand(visuals.expansion / 2.0);
            let stroke = visuals.fg_stroke;
            ui.painter() // paints \
                .line_segment([rect.left_top(), rect.right_bottom()], stroke);
            ui.painter() // paints /
                .line_segment([rect.right_top(), rect.left_bottom()], stroke);
        }

        response
    }
}

pub struct Notifications {
    receiver: Receiver<Toast>,
    toasts: Vec<Toast>,
    next_index: usize,
    id: Id,
}

impl Notifications {
    pub fn new(ctx: &Context) -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();
        ctx.data_mut(|d| d.insert_temp(Id::NULL, NotificationSender(sender)));

        Self {
            receiver,
            toasts: vec![],
            next_index: 0,
            id: Id::new("__notifications"),
        }
    }

    pub fn show(&mut self, ctx: &Context) {
        // Update list of toasts
        match self.receiver.try_recv() {
            Ok(mut toast) => {
                toast.index = self.next_index;
                self.toasts.push(toast);
                self.next_index += 1;
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("channel disconnected!"),
        }
        self.toasts.retain(|toast| toast.ttl_sec > 0.0);

        let dt = ctx.input(|i| i.stable_dt);

        let offsets = self
            .toasts
            .iter()
            .map(|toast| toast.offset)
            .collect::<Vec<_>>();

        let frame_heights = self
            .toasts
            .iter()
            .map(|toast| toast.last_frame_height)
            .collect::<Vec<_>>();

        let len = self.toasts.len();

        let mut shadow = ctx.style().visuals.window_shadow;
        shadow.offset = [4, 4];

        for (i, toast) in self.toasts.iter_mut().enumerate() {
            ctx.request_repaint();

            let response = Area::new(self.id.with("toast").with(toast.index))
                .anchor(
                    Align2::RIGHT_BOTTOM,
                    Vec2::new(-PADDING, -(PADDING + toast.offset)),
                )
                .constrain(false)
                .order(Order::Foreground)
                .interactable(true)
                .show(ctx, |ui| {
                    Frame::window(ui.style())
                        .shadow(shadow)
                        .show(ui, |ui| {
                            let mut top_right = 0.0;
                            let mut bot_right = 0.0;
                            ui.horizontal(|ui| {
                                ui.label(toast.kind.icon_text());
                                ui.vertical(|ui| {
                                    top_right = ui
                                        .label(RichText::new(toast.message.clone()).strong())
                                        .rect
                                        .right();
                                    if let Some(detail) = &toast.detail {
                                        bot_right = ui
                                            .label(RichText::new(detail.clone()).weak())
                                            .rect
                                            .right();
                                    }
                                });
                            });

                            let size = ui.style().spacing.icon_width;
                            let spacing = ui.style().spacing.item_spacing.x;

                            let rect = if top_right + size + spacing < bot_right {
                                Rect::from_min_size(
                                    ui.min_rect().right_top() - Vec2::new(size, 0.0),
                                    Vec2::splat(size),
                                )
                            } else {
                                let rect = Rect::from_min_size(
                                    Pos2::new(top_right + spacing, ui.min_rect().top()),
                                    Vec2::splat(size),
                                );
                                ui.allocate_rect(rect, Sense::hover());
                                rect
                            };

                            toast.progress(ui, rect);
                        })
                        .response
                })
                .response;

            toast.last_frame_height = response.rect.height();

            if !response.contains_pointer() {
                toast.ttl_sec -= dt;
            }

            let force = if i < len - 1 {
                // Ideal positio should never be below 0. Could happen if the new toast is not
                // as tall as expected.
                let ideal = (offsets[i + 1] + frame_heights[i + 1] + PADDING).max(0.0);
                let dist = ideal - toast.offset;
                if toast.offset < ideal {
                    1.5 * dist
                } else {
                    dist
                }
            } else {
                let ideal = FIRST_OFFSET;
                ideal - toast.offset
            };

            toast.velocity += force * dt * SPEED;
            toast.velocity *= COOLOFF;

            toast.offset += toast.velocity * dt * SPEED;
        }
    }
}

pub trait NotifyExt {
    fn notify(
        &self,
        kind: Kind,
        message: impl ToString,
        detail: Option<impl ToString>,
        ttl_sec: f32,
    );

    fn notify_error(&self, message: impl ToString, detail: Option<impl ToString>) {
        self.notify(Kind::Error, message, detail, 8.0);
    }

    fn notify_success(&self, message: impl ToString) {
        self.notify(Kind::Success, message, None::<&str>, 6.0);
    }
}

#[derive(Clone)]
struct NotificationSender(Sender<Toast>);

impl NotifyExt for Context {
    fn notify(
        &self,
        kind: Kind,
        message: impl ToString,
        detail: Option<impl ToString>,
        ttl_sec: f32,
    ) {
        if let Some(NotificationSender(sender)) = self.data(|d| d.get_temp(Id::NULL)) {
            sender
                .send(Toast::new(
                    kind,
                    message.to_string(),
                    detail.map(|d| d.to_string()),
                    ttl_sec,
                ))
                .unwrap();
        }
    }
}
