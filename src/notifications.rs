use egui::{Context, Id, RichText};

#[derive(Clone)]
pub enum Kind {
    Error,
    Success,
}

impl Kind {
    pub fn as_str(&self) -> &str {
        match self {
            Kind::Error => "✘ Error",
            Kind::Success => "✔ Success",
        }
    }
}

#[derive(Clone)]
struct State {
    kind: Kind,
    message: String,
    detail: Option<String>,
    is_open: bool,
}

impl State {
    fn new() -> Self {
        Self {
            kind: Kind::Error,
            message: String::new(),
            detail: None,
            is_open: false,
        }
    }

    fn load(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp(Id::NULL)).unwrap_or_else(Self::new)
    }

    fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Id::NULL, self))
    }

    fn notify(&mut self, kind: Kind, message: String, detail: Option<String>) {
        self.kind = kind;
        self.message = message;
        self.detail = detail;
        self.is_open = true;
    }
}

pub struct Notifications {}

impl Notifications {
    pub fn notify(ctx: &Context, kind: Kind, message: impl ToString, detail: Option<impl ToString>) {
        let mut state = State::load(ctx);
        state.notify(kind, message.to_string(), detail.map(|s| s.to_string()));
        state.store(ctx);
    }

    pub fn error(ctx: &Context, message: impl ToString, detail: Option<impl ToString>) {
        Self::notify(ctx, Kind::Error, message, detail);
    }

    pub fn success(ctx: &Context, message: impl ToString) {
        Self::notify(ctx, Kind::Success, message, None::<String>);
    }

    pub fn show(ctx: &Context) {
        let mut state = State::load(ctx);
        let mut open = state.is_open;
        egui::Window::new(state.kind.as_str())
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(&state.message);
                if let Some(detail) = &state.detail {
                    ui.label(RichText::new(detail).weak());
                }
            });
        state.is_open = open;
        state.store(ctx);
    }
}
