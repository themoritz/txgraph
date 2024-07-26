use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use egui::{Context, CursorIcon, Frame, Key, Pos2, Rect, RichText, Sense, Vec2};

use crate::{
    annotations::Annotations,
    bitcoin::{Transaction, Txid},
    client::Client,
    components::{about::About, custom_tx::CustomTx},
    export::Project,
    flight::Flight,
    framerate::FrameRate,
    graph::Graph,
    layout::Layout,
    loading::Loading,
    notifications::Notifications,
    platform::inner as platform,
    projects::ProjectsWindow,
    style::{Theme, ThemeSwitch},
    transform::Transform,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppStore {
    layout: Layout,
    graph: Graph,
    transform: Transform,
    annotations: Annotations,
    theme: Theme,
    about: About,
    projects: ProjectsWindow,
}

impl AppStore {
    pub fn export(&self) -> String {
        Project::new(&self.graph, &self.annotations).export()
    }
}

pub enum Update {
    LoadOrSelectTx {
        txid: Txid,
        pos: Option<Pos2>,
    },
    SelectTx {
        txid: Txid,
    },
    AddTx {
        txid: Txid,
        tx: Transaction,
        pos: Pos2,
    },
    RemoveTx {
        txid: Txid,
    },
    OpenProject {
        project: Project,
    },
    ExportProject,
}

pub struct App {
    store: AppStore,
    update_sender: Sender<Update>,
    update_receiver: Receiver<Update>,
    flight: Flight,
    ui_size: Vec2,
    custom_tx: CustomTx,
    framerate: FrameRate,
    about_rect: Option<egui::Rect>,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::empty();
        fonts.font_data.insert(
            "btc".to_owned(),
            egui::FontData::from_static(include_bytes!("./fonts/btc.ttf")),
        );
        fonts.font_data.insert(
            "iosevka".to_owned(),
            egui::FontData::from_static(include_bytes!("./fonts/iosevka-custom-regular.ttf")),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name("btc".into()), vec!["btc".to_owned()]);
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "iosevka".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "iosevka".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        let store = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            AppStore::default()
        };

        let (update_sender, update_receiver) = channel();

        platform::add_route_listener(update_sender.clone(), cc.egui_ctx.clone());

        App {
            store,
            update_sender,
            update_receiver,
            flight: Flight::new(),
            ui_size: platform::get_viewport_dimensions().unwrap_or_default(),
            custom_tx: Default::default(),
            framerate: FrameRate::default(),
            about_rect: None,
        }
    }

    pub fn apply_update(&mut self, ctx: &Context, update: Update) {
        match update {
            Update::LoadOrSelectTx { txid, pos } => {
                if let Some(existing_pos) = self.store.graph.get_tx_pos(txid) {
                    self.store.graph.select(txid);
                    self.flight.start(
                        (self.ui_size / 2.0).to_pos2(),
                        self.store.transform.pos_to_screen(existing_pos),
                    );
                    return;
                }

                let center = self.store.transform.pos_from_screen(
                    (self.ui_size / 2.0 + platform::get_random_vec2(50.0)).to_pos2(),
                );

                let sender = self.update_sender.clone();
                let ctx2 = ctx.clone();

                Loading::start_loading_txid(ctx, txid);

                Client::get_json(
                    ctx,
                    &format!("tx/{txid}"),
                    move || {
                        Loading::loading_txid_done(&ctx2, txid);
                    },
                    move |tx| {
                        sender
                            .send(Update::AddTx {
                                txid,
                                tx,
                                pos: pos.unwrap_or(center),
                            })
                            .unwrap();
                        if pos.is_none() {
                            sender.send(Update::SelectTx { txid }).unwrap();
                        }
                    },
                    || {},
                );
            }
            Update::SelectTx { txid } => {
                self.store.graph.select(txid);
                if let Some(pos) = self.store.graph.get_tx_pos(txid) {
                    if let Some(rect) = self.about_rect {
                        if rect.contains(self.store.transform.pos_to_screen(pos)) {
                            self.store.about.close();
                        }
                    }
                }
            }
            Update::AddTx { txid, tx, pos } => {
                self.store.graph.add_tx(txid, tx, pos);
            }
            Update::RemoveTx { txid } => {
                self.store.graph.remove_tx(txid);
            }
            Update::OpenProject { project } => {
                self.store.annotations = project.annotations;

                self.store.graph = Graph::default();
                for tx in &project.transactions {
                    self.update_sender
                        .send(Update::LoadOrSelectTx {
                            txid: tx.txid,
                            pos: Some(tx.position),
                        })
                        .unwrap();
                }

                let num_txs = project.transactions.len();
                let graph_center = if num_txs > 0 {
                    (project
                        .transactions
                        .iter()
                        .fold(Vec2::ZERO, |pos, tx| pos + tx.position.to_vec2())
                        / (num_txs as f32))
                        .to_pos2()
                } else {
                    Pos2::new(0.0, 0.0)
                };
                let screen_center = self
                    .store
                    .transform
                    .pos_from_screen((self.ui_size / 2.0).to_pos2());

                self.store.transform.pan_to(graph_center, screen_center);
            }
            Update::ExportProject => {
                ctx.output_mut(|o| o.copied_text = self.store.export());
            }
        }
        ctx.request_repaint();
    }
}

impl eframe::App for App {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, &self.store);
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.ui_size = platform::get_viewport_dimensions().unwrap_or(ctx.screen_rect().size());

        self.framerate
            .on_new_frame(ctx.input(|i| i.time), frame.info().cpu_usage);

        let sender = self.update_sender.clone();

        let load_tx = |txid: Txid, pos: Option<Pos2>| {
            sender.send(Update::LoadOrSelectTx { txid, pos }).unwrap();
        };

        let frame = Frame::canvas(&ctx.style())
            .inner_margin(0.0)
            .stroke(egui::Stroke::NONE);

        let sender2 = sender.clone();

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.store.about.show_toggle(ui);
                self.store.projects.show_toggle(ui);

                ui.separator();

                ui.menu_button("Tx", |ui| {
                    ui.menu_button("Load Custom Txid", |ui| {
                        self.custom_tx.ui(ui, load_tx);
                    });

                    ui.menu_button("Hallo of Fame", |ui| {
                        ui.allocate_space(Vec2::new(200., 0.));

                        for (name, txid) in Txid::INTERESTING_TXS {
                            if ui.button(name).clicked() {
                                load_tx(Txid::new(txid).unwrap(), None);
                                ui.close_menu();
                            }
                        }

                        ui.separator();
                        ui.label(RichText::new("(from kycp.org)").strong());
                    });
                });

                ui.menu_button("Reset", |ui| {
                    if ui.button("Zoom").clicked() {
                        self.store
                            .transform
                            .reset_zoom((self.ui_size / 2.0).to_pos2());
                        ui.close_menu();
                    }
                    if ui.button("Graph").clicked() {
                        self.store.graph = Graph::default();
                        ui.close_menu();
                    }
                    if ui.button("Annotations").clicked() {
                        self.store.annotations = Annotations::default();
                        ui.close_menu();
                    }
                    if ui.button("All").clicked() {
                        self.store = AppStore::default();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Layout", |ui| {
                    self.store.layout.ui(ui);
                });

                ui.add(ThemeSwitch::new(&mut self.store.theme));

                Loading::spinner(ui);
            });
        });

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            let mut response = ui.allocate_response(
                ui.available_size_before_wrap(),
                Sense::click_and_drag().union(Sense::hover()),
            );

            self.framerate.ui(&mut ui.child_ui(
                Rect::from_min_max(
                    response.rect.right_top() - Vec2::new(-10., -5.),
                    response.rect.right_top() + Vec2::new(-5., 10.),
                ),
                egui::Layout::right_to_left(egui::Align::Min),
            ));

            ui.set_clip_rect(response.rect);

            if self.flight.is_active() {
                let delta = self.flight.update();
                self.store.transform.translate(-delta);
                ctx.request_repaint();
            }

            // Zoom
            if let Some(hover_pos) = response.hover_pos() {
                let zoom_delta = ui.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 {
                    self.store.transform.zoom(zoom_delta, hover_pos);
                    self.flight.interrupt();
                }

                let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
                if scroll_delta.y != 0.0 {
                    self.store
                        .transform
                        .zoom(1.0 + scroll_delta.y / 200.0, hover_pos);
                    self.flight.interrupt();
                }
            }

            // Drag
            if response.dragged_by(egui::PointerButton::Primary) {
                response = response.on_hover_cursor(CursorIcon::Grabbing);
                self.store.transform.translate(response.drag_delta());
                self.flight.interrupt();
            }

            let mut pan = Vec2::ZERO;
            if ui.input(|i| i.key_down(Key::ArrowDown)) {
                pan += Vec2::DOWN;
            }
            if ui.input(|i| i.key_down(Key::ArrowUp)) {
                pan += Vec2::UP;
            }
            if ui.input(|i| i.key_down(Key::ArrowLeft)) {
                pan += Vec2::LEFT;
            }
            if ui.input(|i| i.key_down(Key::ArrowRight)) {
                pan += Vec2::RIGHT;
            }
            if pan != Vec2::ZERO {
                self.store.transform.translate(pan * 2.);
                self.flight.interrupt();
                ctx.request_repaint();
            }

            loop {
                match self.update_receiver.try_recv() {
                    Ok(update) => self.apply_update(ctx, update),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => panic!("channel disconnected!"),
                }
            }

            self.store.graph.draw(
                ui,
                &self.store.transform,
                sender2,
                &self.store.layout,
                &mut self.store.annotations,
            );
        });

        let sender2 = sender.clone();
        self.store.projects.show_window(
            ctx,
            |project| {
                sender.send(Update::OpenProject { project }).unwrap();
            },
            || {
                sender2.send(Update::ExportProject).unwrap();
            },
            || {
                Project::new(&self.store.graph, &self.store.annotations)
            }
        );
        self.about_rect = self.store.about.show_window(ctx, load_tx);

        Notifications::show(ctx);
    }
}
