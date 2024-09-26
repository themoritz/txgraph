use egui::{Align2, Area, Color32, Context, Frame, Id, Order, Pos2, RichText, Sense, Ui, Vec2};

pub fn show(ctx: &Context, title: impl Into<RichText>, add_contents: impl FnOnce(&mut Ui)) {
    let rect = ctx.screen_rect();

    Area::new(Id::new("Modal"))
        .fixed_pos(Pos2::ZERO)
        .movable(false)
        .order(Order::Foreground)
        .show(ctx, |ui| {
            let response = ui.interact(rect, Id::new("Model response"), Sense::click());
            ui.painter()
                .rect_filled(rect, 0.0, Color32::from_black_alpha(32));
            response
        });

    let title: RichText = title.into();
    let id = Id::new("Modal").with(title.text());

    Area::new(Id::new(id))
        .anchor(Align2::CENTER_CENTER, Vec2::new(0.0, -rect.height() / 8.0))
        .movable(false)
        .order(Order::Debug) // TODO: this seems like a hack, how do I get the modal to be on top?
        .show(ctx, |ui| {
            Frame::popup(&ctx.style()).show(ui, |ui| {
                ui.heading(title);
                add_contents(ui);
            });
        });
}
