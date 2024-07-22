use egui::{Align2, Area, Color32, Context, Frame, Id, Order, RichText, Sense, Ui, Vec2};

pub fn show(ctx: &Context, title: impl Into<RichText>, add_contents: impl FnOnce(&mut Ui)) {
    Area::new(Id::new("Modal"))
        .anchor(Align2::LEFT_TOP, Vec2::new(0.0, -30.0))
        .movable(false)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            let response = ui.interact(
                ui.available_rect_before_wrap(),
                Id::new("Model response"),
                Sense::click(),
            );
            ui.painter().rect_filled(
                ui.available_rect_before_wrap(),
                0.0,
                Color32::from_black_alpha(128),
            );
            response
        });

    let title: RichText = title.into();
    let id = Id::new("Modal").with(title.text());

    Area::new(Id::new(id))
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -100.0))
        .movable(false)
        .order(Order::Debug) // TODO: this seems like a hack, how do I get the modal to be on top?
        .show(ctx, |ui| {
            Frame::popup(&ctx.style()).show(ui, |ui| {
                ui.heading(title);
                add_contents(ui);
            });
        });
}
