use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use egui::{
    ahash::HashSet, Button, CursorIcon, Frame, Key, Pos2, Rect, RichText, Sense, TextEdit,
    TextStyle, Vec2,
};

use crate::{
    annotations::Annotations,
    bitcoin::{Transaction, Txid},
    export::Project,
    flight::Flight,
    framerate::FrameRate,
    graph::Graph,
    layout::Layout,
    platform::inner as platform,
    style::{Theme, ThemeSwitch},
    transform::Transform,
    widgets::BulletPoint,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppStore {
    tx: String,
    layout: Layout,
    graph: Graph,
    transform: Transform,
    annotations: Annotations,
    theme: Theme,
}

impl AppStore {
    pub fn export(&self) -> String {
        Project::new(&self.graph, &self.annotations).export()
    }
}

impl Default for AppStore {
    fn default() -> Self {
        AppStore {
            tx: "".to_owned(),
            layout: Default::default(),
            graph: Default::default(),
            transform: Default::default(),
            annotations: Default::default(),
            theme: Default::default(),
        }
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
    Loading {
        txid: Txid,
    },
    LoadingDone {
        txid: Txid,
    },
    Error {
        err: String,
    },
}

pub struct App {
    store: AppStore,
    update_sender: Sender<Update>,
    update_receiver: Receiver<Update>,
    err: String,
    err_open: bool,
    loading: HashSet<Txid>,
    flight: Flight,
    ui_size: Vec2,
    import_text: String,
    framerate: FrameRate,
    about_open: bool,
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
            err: String::new(),
            err_open: false,
            flight: Flight::new(),
            ui_size: platform::get_viewport_dimensions().unwrap_or_default(),
            loading: HashSet::default(),
            import_text: String::new(),
            framerate: FrameRate::default(),
            about_open: true,
            about_rect: None,
        }
    }

    pub fn apply_update(&mut self, update: Update) {
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

                let request = ehttp::Request::get(format!("https://txgraph.info/api/tx/{}", txid));
                self.update_sender.send(Update::Loading { txid }).unwrap();

                let sender = self.update_sender.clone();
                let center = self.store.transform.pos_from_screen(
                    (self.ui_size / 2.0 + platform::get_random_vec2(50.0)).to_pos2(),
                );

                ehttp::fetch(request, move |response| {
                    sender.send(Update::LoadingDone { txid }).unwrap();
                    let error = |e: String| sender.send(Update::Error { err: e }).unwrap();
                    match response {
                        Ok(response) => {
                            if response.status == 200 {
                                if let Some(text) = response.text() {
                                    match serde_json::from_str(text) {
                                        Ok(tx) => {
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
                                        }
                                        Err(err) => {
                                            error(err.to_string());
                                        }
                                    }
                                } else {
                                    error("No text body response".to_string());
                                }
                            } else {
                                error(response.text().map_or("".to_string(), |t| t.to_owned()));
                            }
                        }
                        Err(err) => {
                            error(err);
                        }
                    }
                });
            }
            Update::SelectTx { txid } => {
                self.store.graph.select(txid);
                if let Some(pos) = self.store.graph.get_tx_pos(txid) {
                    if let Some(rect) = self.about_rect {
                        if rect.contains(self.store.transform.pos_to_screen(pos)) {
                            self.about_open = false;
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
            Update::Loading { txid } => {
                self.loading.insert(txid);
            }
            Update::LoadingDone { txid } => {
                self.loading.remove(&txid);
            }
            Update::Error { err } => {
                self.err = err;
                self.err_open = true
            }
        }
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
                if ui.selectable_label(self.about_open, "About").clicked() {
                    self.about_open = !self.about_open;
                }

                ui.separator();

                ui.menu_button("Project", |ui| {
                    if ui.button("Export to Clipboard").clicked() {
                        ui.output_mut(|o| o.copied_text = self.store.export());
                        ui.close_menu();
                    }
                    ui.menu_button("Import", |ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.import_text).hint_text("Paste JSON..."),
                        );
                        if ui.button("Go").clicked() {
                            match Project::import(&self.import_text) {
                                Ok(project) => {
                                    self.store.annotations = project.annotations;

                                    self.store.graph = Graph::default();
                                    for tx in &project.transactions {
                                        load_tx(tx.txid, Some(tx.position));
                                    }

                                    let num_txs = project.transactions.len() as f32;
                                    let graph_center = (project
                                        .transactions
                                        .iter()
                                        .fold(Vec2::ZERO, |pos, tx| pos + tx.position.to_vec2())
                                        / num_txs)
                                        .to_pos2();
                                    let screen_center = self
                                        .store
                                        .transform
                                        .pos_from_screen((self.ui_size / 2.0).to_pos2());

                                    self.store.transform.pan_to(graph_center, screen_center);

                                    self.import_text = String::new();
                                }
                                Err(e) => sender
                                    .send(Update::Error {
                                        err: format!("Could not import Json because:\n{}", e),
                                    })
                                    .unwrap(),
                            }
                            ui.close_menu();
                        }
                    });
                });

                ui.menu_button("Tx", |ui| {
                    ui.menu_button("Load Custom Txid", |ui| {
                        let glyph_width =
                            ui.fonts(|f| f.glyph_width(&TextStyle::Body.resolve(ui.style()), '0'));
                        ui.allocate_space(Vec2::new(glyph_width * 63.5, 0.0));

                        ui.add(
                            TextEdit::singleline(&mut self.store.tx)
                                .hint_text("Enter Txid")
                                .desired_width(f32::INFINITY),
                        );

                        ui.horizontal(|ui| match Txid::new(&self.store.tx) {
                            Ok(txid) => {
                                if ui.button("Go").clicked() {
                                    load_tx(txid, None);
                                    ui.close_menu();
                                }
                            }
                            Err(e) => {
                                ui.add_enabled(false, Button::new("Go"));
                                ui.label(format!("Invalid Txid: {}", e));
                            }
                        });
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

                if !self.loading.is_empty() {
                    ui.spinner();
                }
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
                    Ok(update) => self.apply_update(update),
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
                &self.loading,
            );
        });

        let response = egui::Window::new("txgraph.info")
            .open(&mut self.about_open)
            .show(ctx, |ui| {
                ui.label("Visualizing Bitcoin's transaction graph.");

                if ui.button("Load Example Transaction").clicked() {
                    load_tx(Txid::random_interesting(), None);
                }

                egui::CollapsingHeader::new("Instructions")
                    .default_open(true)
                    .show(ui, |ui| {
                        let steps = [
                            "Load a custom transaction or pick one from the Hall of Fame via the 'Tx' menu.",
                            "Click on inputs / outputs to expand to the next transaction.",
                            "Drag/pinch screen to pan/zoom.",
                            "Drag transactions to adjust layout.",
                            "Right-click transactions or inputs/outputs.",
                        ];

                        for step in steps {
                            ui.add(BulletPoint::new(step));
                        }
                    });

                ui.add_space(3.0);

                ui.horizontal(|ui| {
                    ui.hyperlink_to("GitHub", "https://github.com/themoritz/txgraph");
                    ui.label("⸱");
                    ui.hyperlink_to("Contact", "mailto:hello@txgraph.info");
                });
            });

        self.about_rect = response.map(|r| r.response.rect);

        egui::Window::new("Error")
            .open(&mut self.err_open)
            .show(ctx, |ui| {
                ui.label(&self.err);
            });
    }
}
