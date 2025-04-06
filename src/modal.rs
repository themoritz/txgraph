use egui::{Color32, Context, Id, Modal, RichText, Ui};

pub fn show(ctx: &Context, title: impl Into<RichText>, add_contents: impl FnOnce(&mut Ui)) {
    let title: RichText = title.into();
    let id = Id::new("Modal").with(title.text());

    Modal::new(id)
        .backdrop_color(Color32::from_black_alpha(32))
        .show(ctx, |ui| {
            ui.heading(title);
            add_contents(ui);
        });
}
