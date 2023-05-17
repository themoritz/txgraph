use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};

use egui::{Button, CursorIcon, Frame, Grid, Pos2, Sense, TextEdit, TextStyle, Vec2};

use crate::{
    annotations::Annotations,
    bitcoin::{Transaction, Txid},
    export::Project,
    graph::Graph,
    transform::Transform,
    widgets::BulletPoint,
};

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct AppStore {
    tx: String,
    layout_params: LayoutParams,
    graph: Graph,
    transform: Transform,
    annotations: Annotations,
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
            layout_params: Default::default(),
            graph: Default::default(),
            transform: Default::default(),
            annotations: Default::default(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct LayoutParams {
    pub scale: f32,
    pub dt: f32,
    pub cooloff: f32,
    pub y_compress: f32,
    pub tx_repulsion_dropoff: f32,
}

impl Default for LayoutParams {
    fn default() -> Self {
        Self {
            scale: 80.0,
            dt: 0.08,
            cooloff: 0.85,
            y_compress: 2.0,
            tx_repulsion_dropoff: 1.2,
        }
    }
}

pub enum Update {
    AddTx {
        txid: Txid,
        tx: Transaction,
        pos: Pos2,
    },
    RemoveTx {
        txid: Txid,
    },
    Loading,
    LoadingDone,
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
    loading: bool,
    import_text: String,
}

impl App {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.
        cc.egui_ctx.set_visuals(egui::Visuals::light());

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

        App {
            store,
            update_sender,
            update_receiver,
            err: String::new(),
            err_open: false,
            loading: false,
            import_text: String::new(),
        }
    }

    pub fn apply_update(&mut self, update: Update) {
        match update {
            Update::AddTx { txid, tx, pos } => {
                self.store.graph.add_tx(txid, tx, pos);
            }
            Update::RemoveTx { txid } => {
                self.store.graph.remove_tx(txid);
            }
            Update::Loading => self.loading = true,
            Update::LoadingDone => self.loading = false,
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

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        loop {
            match self.update_receiver.try_recv() {
                Ok(update) => self.apply_update(update),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => panic!("channel disconnected!"),
            }
        }

        let sender = self.update_sender.clone();

        let remove_tx = |txid: Txid| {
            sender.send(Update::RemoveTx { txid }).unwrap();
        };

        let load_tx = |txid: Txid, pos: Pos2| {
            let request = ehttp::Request::get(format!("https://txgraph.info/tx/{}", txid));
            sender.send(Update::Loading).unwrap();

            let ctx = ctx.clone();
            let sender = sender.clone();

            ehttp::fetch(request, move |response| {
                sender.send(Update::LoadingDone).unwrap();
                let error = |e: String| sender.send(Update::Error { err: e }).unwrap();
                match response {
                    Ok(response) => {
                        if response.status == 200 {
                            if let Some(text) = response.text() {
                                match serde_json::from_str(text) {
                                    Ok(tx) => {
                                        sender.send(Update::AddTx { txid, tx, pos }).unwrap();
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
                ctx.request_repaint();
            });
        };

        let frame = Frame::canvas(&ctx.style()).inner_margin(0.0);
        ctx.request_repaint();

        let mut ui_size = Vec2::ZERO;

        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui_size = ui.available_size_before_wrap();

            let mut response = ui.allocate_response(
                ui.available_size_before_wrap(),
                Sense::click_and_drag().union(Sense::hover()),
            );

            // Zoom
            if let Some(hover_pos) = response.hover_pos() {
                let zoom_delta = ui.input(|i| i.zoom_delta());
                if zoom_delta != 1.0 {
                    self.store.transform.zoom(zoom_delta, hover_pos);
                }
            }

            // Drag
            if response.dragged_by(egui::PointerButton::Primary) {
                response = response.on_hover_cursor(CursorIcon::Grabbing);
                self.store.transform.translate(response.drag_delta());
            }

            self.store.graph.draw(
                ui,
                &self.store.transform,
                load_tx,
                remove_tx,
                &self.store.layout_params,
                &mut self.store.annotations,
            );
        });

        egui::Window::new("txgraph.info").show(ctx, |ui| {
            let screen_center = self
                .store
                .transform
                .pos_from_screen((ui_size / 2.0).to_pos2());

            ui.horizontal(|ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Export to Clipboard").clicked() {
                        ui.output_mut(|o| o.copied_text = self.store.export());
                        ui.close_menu();
                    }
                    ui.menu_button("Import", |ui| {
                        ui.add(
                            TextEdit::singleline(&mut self.import_text).hint_text("Paste JSON..."),
                        );
                        if ui.button("Go").clicked() {
                            match Project::import_(&self.import_text) {
                                Ok((annotations, transactions)) => {
                                    self.store.annotations = annotations;

                                    self.store.graph = Graph::default();
                                    for tx in &transactions {
                                        load_tx(tx.txid, tx.position.to_pos2());
                                    }

                                    let num_txs = transactions.len() as f32;
                                    let center =
                                        (transactions.iter().fold(Vec2::ZERO, |pos, tx| {
                                            pos + tx.position.to_pos2().to_vec2()
                                        }) / num_txs)
                                            .to_pos2();
                                    self.store.transform.pan_to(center, screen_center);

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

                ui.menu_button("Reset", |ui| {
                    if ui.button("Zoom").clicked() {
                        self.store.transform.reset_zoom((ui_size / 2.0).to_pos2());
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
                });
                if self.loading {
                    ui.spinner();
                }
            });

            ui.allocate_space(Vec2::new(300.0, 3.0));

            egui::CollapsingHeader::new("Instructions")
                .default_open(true)
                .show(ui, |ui| {
                    let steps = [
                        "Load a custom transaction or pick one from the Hall of Fame.",
                        "Click on inputs / outputs to expand to the next transaction.",
                        "Drag/pinch screen to pan/zoom.",
                        "Drag transactions to adjust layout.",
                        "Right-click transactions or inputs/outputs.",
                    ];

                    for step in steps {
                        ui.add(BulletPoint::new(step));
                    }
                });

            ui.collapsing("Custom Transaction", |ui| {
                let glyph_width =
                    ui.fonts(|f| f.glyph_width(&TextStyle::Body.resolve(ui.style()), '0'));
                ui.add(
                    TextEdit::singleline(&mut self.store.tx)
                        .hint_text("Enter Txid")
                        .desired_width(glyph_width * 63.5),
                );

                ui.horizontal(|ui| match Txid::new(&self.store.tx) {
                    Ok(txid) => {
                        if ui.button("Go").clicked() {
                            load_tx(txid, screen_center);
                        }
                    }
                    Err(e) => {
                        ui.add_enabled(false, Button::new("Go"));
                        ui.label(format!("Invalid Txid: {}", e));
                    }
                });
            });

            egui::CollapsingHeader::new("Hall of Fame (from kycp.org)")
                .default_open(true)
                .show(ui, |ui| {
                    let interesting_txs = vec![
                        (
                            "First Bitcoin",
                            "0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098",
                        ),
                        (
                            "First TX (Satoshi to Hal Finney)",
                            "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16",
                        ),
                        (
                            "10.000 BTC pizza",
                            "a1075db55d416d3ca199f55b6084e2115b9345e16c5cf302fc80e9d5fbf5d48d",
                        ),
                        (
                            "Whirlpool",
                            "323df21f0b0756f98336437aa3d2fb87e02b59f1946b714a7b09df04d429dec2",
                        ),
                        (
                            "Wasabi",
                            "b3dcc5d68e7ba4946e8e7fec0207906fba89ccb4768112a25d6e6941f2e99d97",
                        ),
                        (
                            "Wasabi post-mix spending",
                            "4f89d6599fd1d728a78972d96930b8fca55e060aca9a04171b6c703c88285325",
                        ),
                        (
                            "DarkWallet",
                            "8e56317360a548e8ef28ec475878ef70d1371bee3526c017ac22ad61ae5740b8",
                        ),
                        (
                            "MTGox 424242.42424242",
                            "3a1b9e330d32fef1ee42f8e86420d2be978bbe0dc5862f17da9027cf9e11f8c4",
                        ),
                        (
                            "Basic transaction",
                            "2f17c08654e518f3ee46dd1438b58ef52b772e8cbc446b96b123d680a80bc3f7",
                        ),
                        (
                            "Non-deterministic TX",
                            "015d9cf0a12057d009395710611c65109f36b3eaefa3a694594bf243c097f404",
                        ),
                        (
                            "Complex TX",
                            "722d83ae4183ee17704704bdf31d9e77e6964387f657bbc0e09810a84a7fbad2",
                        ),
                        (
                            "JoinMarket",
                            "ca48b14f0a836b91d8719c51e50b313b425356a87111c4ed2cd6d81f0dbe60de",
                        ),
                        (
                            "Weak CoinJoin",
                            "a9b5563592099bf6ed68e7696eeac05c8cb514e21490643e0b7a9b72dac90b07",
                        ),
                        (
                            "Address reuse",
                            "0f7bf562c8768454077f9b5c6fe0c4c55c9a34786ad7380e00c2d8d00ebf779d",
                        ),
                        (
                            "Block reward",
                            "2157b554dcfda405233906e461ee593875ae4b1b97615872db6a25130ecc1dd6",
                        ),
                        (
                            "Input/output merges",
                            "03a858678475235b8b35a67495d67b65d5f2323236571aba3395f57eac57d72d",
                        ),
                        (
                            "Multisig + address reuse",
                            "dbbd98e638cc69a771fff79b34f5c6d59f08366f2238472c82d68b63757e051a",
                        ),
                        (
                            "Taproot",
                            "83c8e0289fecf93b5a284705396f5a652d9886cbd26236b0d647655ad8a37d82",
                        ),
                    ];

                    for (name, txid) in interesting_txs {
                        if ui.button(name).clicked() {
                            load_tx(Txid::new(txid).unwrap(), screen_center);
                        }
                    }
                });

            ui.collapsing("Layout Parameters", |ui| {
                Grid::new("Layout").num_columns(2).show(ui, |ui| {
                    ui.label("Scale:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.scale,
                        5.0..=200.0,
                    ));
                    ui.end_row();

                    ui.label("Y Compress:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.y_compress,
                        1.0..=5.0,
                    ));
                    ui.end_row();

                    ui.label("Tx repulsion factor:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.tx_repulsion_dropoff,
                        0.5..=2.0,
                    ));
                    ui.end_row();

                    ui.label("Speed:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.dt,
                        0.001..=0.2,
                    ));
                    ui.end_row();

                    ui.label("Cooloff:");
                    ui.add(egui::Slider::new(
                        &mut self.store.layout_params.cooloff,
                        0.5..=0.99,
                    ));
                    ui.end_row();
                });
            });

            ui.add_space(3.0);

            ui.horizontal(|ui| {
                ui.add_space(75.0);
                ui.hyperlink_to("GitHub", "https://github.com/themoritz/txgraph");
                ui.label("⸱");
                ui.hyperlink_to("Contact", "mailto:hello@txgraph.info");
            });
        });

        egui::Window::new("Error")
            .open(&mut self.err_open)
            .show(ctx, |ui| {
                ui.label(&self.err);
            });
    }
}
