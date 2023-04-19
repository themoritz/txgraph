use std::{
    collections::{HashMap, HashSet},
    f32::consts::PI,
};

use eframe::epaint::TextShape;
use egui::{
    show_tooltip_at_pointer, text::LayoutJob, Align, Color32, FontId, Pos2, Rect, RichText,
    Rounding, Sense, Stroke, TextFormat, Vec2,
};

use crate::{
    app::LayoutParams,
    bezier::Edge,
    bitcoin::{AddressType, AmountComponents, Sats, Transaction, Txid},
    components::Components,
    transform::Transform,
};

pub struct DrawableGraph {
    nodes: HashMap<Txid, DrawableNode>,
    edges: Vec<DrawableEdge>,
    components: Components,
}

pub struct DrawableNode {
    /// Center of tx rect.
    pos: Pos2,
    velocity: Vec2,
    dragged: bool,
    height: f32,
    tx_value: u64,
    tx_timestamp: String,
    block_height: u32,
    inputs: Vec<DrawableInput>,
    outputs: Vec<DrawableOutput>,
}

#[derive(Clone, Hash)]
pub struct DrawableEdge {
    source: Txid,
    source_pos: usize,
    target: Txid,
    target_pos: usize,
}

pub struct DrawableInput {
    top: f32,
    bot: f32,
    value: u64,
    address: String,
    address_type: AddressType,
    funding_txid: Txid, // TODO: coinbase tx?
    funding_vout: u32,
}

pub struct DrawableOutput {
    top: f32,
    bot: f32,
    value: u64,
    output_type: OutputType,
}

pub enum OutputType {
    Utxo {
        address: String,
        address_type: AddressType,
    },
    Spent {
        spending_txid: Txid,
        address: String,
        address_type: AddressType,
    },
    Fees,
}

impl DrawableGraph {
    pub fn empty() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            components: Components::new(),
        }
    }

    fn add_edge(&mut self, edge: DrawableEdge) {
        self.components.connect(edge.source, edge.target);
        self.edges.push(edge);
    }

    pub fn remove_tx(&mut self, txid: Txid) {
        self.nodes.remove(&txid);
        self.edges
            .retain(|edge| edge.source != txid && edge.target != txid);

        // Recreate connected components
        self.components = Components::new();
        for edge in &self.edges {
            self.components.connect(edge.source, edge.target);
        }
    }

    pub fn add_tx(&mut self, txid: Txid, tx: Transaction, pos: Pos2) {
        // Add node
        fn scale(value: u64) -> f32 {
            f32::powf(value as f32, 1.0 / 3.0).round() / 10.0
        }

        let height = scale(tx.amount());

        let input_height: f32 = tx.inputs.iter().map(|input| scale(input.value)).sum();

        let output_height = tx
            .outputs
            .iter()
            .map(|output| scale(output.value))
            .sum::<f32>()
            + scale(tx.fees());

        let mut bot = 0.0;

        let inputs = tx
            .inputs
            .iter()
            .map(|i| {
                let h = scale(i.value) * height / input_height;
                bot += h;
                DrawableInput {
                    top: bot - h,
                    bot,
                    value: i.value,
                    address: i.address.clone(),
                    address_type: i.address_type,
                    funding_txid: i.txid,
                    funding_vout: i.vout,
                }
            })
            .collect();

        bot = 0.0;

        let mut outputs: Vec<DrawableOutput> = tx
            .outputs
            .iter()
            .map(|o| {
                let h = scale(o.value) * height / output_height;
                bot += h;
                DrawableOutput {
                    top: bot - h,
                    bot,
                    value: o.value,
                    output_type: match o.spending_txid {
                        None => OutputType::Utxo {
                            address: o.address.clone(),
                            address_type: o.address_type,
                        },
                        Some(txid) => OutputType::Spent {
                            spending_txid: txid,
                            address: o.address.clone(),
                            address_type: o.address_type,
                        },
                    },
                }
            })
            .collect();

        outputs.push(DrawableOutput {
            top: bot,
            bot: bot + scale(tx.fees()) * height / output_height,
            value: tx.fees(),
            output_type: OutputType::Fees,
        });

        self.nodes.insert(
            txid,
            DrawableNode {
                pos,
                velocity: Vec2::new(0.0, 0.0),
                dragged: false,
                height,
                tx_value: tx.amount(),
                tx_timestamp: chrono::NaiveDateTime::from_timestamp_opt(tx.timestamp, 0)
                    .unwrap()
                    .format("%Y-%m-%d %H:%M:%S")
                    .to_string(),
                block_height: tx.block_height,
                inputs,
                outputs,
            },
        );

        // Add edges
        for (i, input) in tx.inputs.iter().enumerate() {
            if self.nodes.contains_key(&input.txid) {
                self.add_edge(DrawableEdge {
                    source: input.txid,
                    source_pos: input.vout as usize,
                    target: txid,
                    target_pos: i,
                });
            }
        }

        for (o, output) in tx.outputs.iter().enumerate() {
            if let Some(spending_txid) = output.spending_txid {
                if let Some(spending_tx) = self.nodes.get(&spending_txid) {
                    let target_pos = spending_tx
                        .inputs
                        .iter()
                        .enumerate()
                        .find(|(_, inp)| inp.funding_txid == txid && inp.funding_vout as usize == o)
                        .unwrap()
                        .0;
                    self.add_edge(DrawableEdge {
                        source: txid,
                        source_pos: o,
                        target: spending_txid,
                        target_pos,
                    });
                }
            }
        }
    }

    pub fn draw(
        &mut self,
        ui: &egui::Ui,
        transform: &Transform,
        load_tx: impl Fn(Txid, Pos2),
        remove_tx: impl Fn(Txid),
        layout_params: &LayoutParams,
    ) {
        const TX_WIDTH: f32 = 36.0;
        const IO_WIDTH: f32 = 10.0;
        let scale2 = layout_params.scale * layout_params.scale;

        let font_id = FontId::monospace(10.0);
        let format = TextFormat {
            font_id: font_id.clone(),
            color: Color32::BLACK,
            ..Default::default()
        };

        let initial_dist = Vec2::new(IO_WIDTH + TX_WIDTH / 2.0 + 5.0, 0.0);

        let painter = ui.painter();

        let mut input_rects: HashMap<(Txid, usize), Rect> = HashMap::new();
        let mut output_rects: HashMap<(Txid, usize), Rect> = HashMap::new();

        let rects: HashMap<Txid, Rect> = self
            .nodes
            .iter()
            .map(|(t, n)| {
                (
                    *t,
                    Rect::from_center_size(n.pos, Vec2::new(TX_WIDTH + 2.0 * IO_WIDTH, n.height)),
                )
            })
            .collect();

        let txids: HashSet<Txid> = self.nodes.keys().map(|t| *t).collect();

        for (txid, node) in &mut self.nodes {
            let top_left = node.pos - Vec2::new(TX_WIDTH / 2.0, node.height / 2.0);
            let rect = transform.rect_to_screen(Rect::from_min_size(
                top_left,
                Vec2::new(TX_WIDTH, node.height),
            ));
            let response = ui
                .interact(rect, ui.id().with(txid), Sense::drag())
                .on_hover_ui(|ui| {
                    ui.label(RichText::new("Transaction").heading().monospace());
                    let mut job = LayoutJob::default();
                    txid_layout(&mut job, &txid, &font_id);
                    newline(&mut job, &font_id);
                    newline(&mut job, &FontId::monospace(5.0));
                    sats_layout(&mut job, &Sats(node.tx_value), &font_id);
                    job.append(
                        &format!("\n{} (block {})", node.tx_timestamp, node.block_height),
                        0.0,
                        format.clone(),
                    );
                    ui.label(job);
                });

            if response.dragged() {
                node.dragged = true;
                node.velocity = Vec2::ZERO;
                node.pos += transform.vec_from_screen(response.drag_delta());
            } else {
                node.dragged = false;
            }

            painter.rect(
                rect,
                Rounding::none(),
                Color32::LIGHT_GREEN.gamma_multiply(0.8),
                Stroke::new(1.0, Color32::BLACK),
            );

            let tx_painter = painter.with_clip_rect(rect);
            tx_painter.add(rotated_layout(
                ui,
                tx_content(&txid, &node.tx_timestamp, &Sats(node.tx_value)),
                rect.right_top() + Vec2::new(-1.0, 2.0),
                PI / 2.0,
            ));

            let id = ui.id().with("i").with(txid);
            for (i, input) in node.inputs.iter().enumerate() {
                let rect = Rect::from_min_max(
                    Pos2::new(top_left.x - IO_WIDTH, top_left.y + input.top),
                    Pos2::new(top_left.x, top_left.y + input.bot),
                );
                let screen_rect = transform.rect_to_screen(rect);
                let response = ui
                    .interact(screen_rect, id.with(i), Sense::click())
                    .on_hover_ui(|ui| {
                        ui.label(RichText::new("⏴Input").heading().monospace());
                        let mut job = LayoutJob::default();
                        sats_layout(&mut job, &Sats(input.value), &font_id);
                        newline(&mut job, &font_id);
                        address_layout(&mut job, &input.address, input.address_type, &font_id);
                        newline(&mut job, &font_id);
                        newline(&mut job, &FontId::monospace(5.0));
                        txid_layout(&mut job, &input.funding_txid, &font_id);
                        ui.label(job);
                    });

                if response.clicked() {
                    if txids.contains(&input.funding_txid) {
                        remove_tx(input.funding_txid);
                    } else {
                        load_tx(input.funding_txid, rect.left_center() - initial_dist);
                    }
                }

                painter.rect(
                    screen_rect,
                    Rounding::none(),
                    Color32::WHITE.gamma_multiply(0.8),
                    ui.style().interact(&response).fg_stroke,
                );

                input_rects.insert((*txid, i), rect);
            }

            let id = ui.id().with("o").with(txid);
            for (o, output) in node.outputs.iter().enumerate() {
                let rect = Rect::from_min_max(
                    Pos2::new(top_left.x + TX_WIDTH, top_left.y + output.top),
                    Pos2::new(top_left.x + TX_WIDTH + IO_WIDTH, top_left.y + output.bot),
                );
                let screen_rect = transform.rect_to_screen(rect);
                let response = ui
                    .interact(screen_rect, id.with(o), Sense::click())
                    .on_hover_ui(|ui| match &output.output_type {
                        OutputType::Utxo {
                            address,
                            address_type,
                        } => {
                            ui.label(RichText::new("Unspent Output").heading().monospace());
                            let mut job = LayoutJob::default();
                            sats_layout(&mut job, &Sats(output.value), &font_id);
                            newline(&mut job, &font_id);
                            address_layout(&mut job, &address, *address_type, &font_id);
                            ui.label(job);
                        }
                        OutputType::Spent {
                            spending_txid,
                            address,
                            address_type,
                        } => {
                            ui.label(RichText::new("Output⏵").heading().monospace());
                            let mut job = LayoutJob::default();
                            sats_layout(&mut job, &Sats(output.value), &font_id);
                            newline(&mut job, &font_id);
                            address_layout(&mut job, &address, *address_type, &font_id);
                            newline(&mut job, &font_id);
                            newline(&mut job, &FontId::monospace(5.0));
                            txid_layout(&mut job, &spending_txid, &font_id);
                            ui.label(job);
                        }
                        OutputType::Fees => {
                            ui.label(RichText::new("Fees").heading().monospace());
                            let mut job = LayoutJob::default();
                            sats_layout(&mut job, &Sats(output.value), &font_id);
                            ui.label(job);
                        }
                    });

                if let OutputType::Spent {
                    spending_txid,
                    address: _,
                    address_type: _,
                } = &output.output_type
                {
                    if response.clicked() {
                        if txids.contains(spending_txid) {
                            remove_tx(*spending_txid);
                        } else {
                            load_tx(*spending_txid, rect.right_center() + initial_dist);
                        }
                    }
                }

                painter.rect(
                    screen_rect,
                    Rounding::none(),
                    match output.output_type {
                        OutputType::Utxo {
                            address: _,
                            address_type: _,
                        } => Color32::GRAY.gamma_multiply(0.8),
                        OutputType::Spent {
                            spending_txid: _,
                            address: _,
                            address_type: _,
                        } => Color32::WHITE.gamma_multiply(0.8),
                        OutputType::Fees => Color32::BLACK.gamma_multiply(0.8),
                    },
                    ui.style().interact(&response).fg_stroke,
                );

                output_rects.insert((*txid, o), rect);
            }

            // Calculate repulsion force between txs and update velocity
            for (other_txid, other_rect) in &rects {
                if *other_txid == *txid {
                    continue;
                }
                let this_rect = rects.get(txid).unwrap();
                let diff = other_rect.center() - this_rect.center();
                let spacing = clear_spacing(this_rect, other_rect);
                let force =
                    -scale2 / spacing.powf(layout_params.tx_repulsion_dropoff) * diff.normalized();

                // Repulsion does not apply across connected components if the nodes aren't close to each other.
                if self.components.connected(*txid, *other_txid)
                    || spacing <= 0.5 * layout_params.scale
                {
                    node.velocity += force * layout_params.dt;
                }
            }
        }

        for edge in &self.edges {
            let from_rect = output_rects.get(&(edge.source, edge.source_pos)).unwrap();
            let to_rect = input_rects.get(&(edge.target, edge.target_pos)).unwrap();

            let flow = Edge {
                from: from_rect.right_top(),
                from_height: from_rect.height(),
                to: to_rect.left_top(),
                to_height: to_rect.height(),
            };

            if flow.draw(&ui, &transform).hovering {
                let id = ui.id().with("edge").with(edge);
                show_tooltip_at_pointer(ui.ctx(), id, |ui| {
                    let input = &self.nodes.get(&edge.target).unwrap().inputs[edge.target_pos];
                    let mut job = LayoutJob::default();
                    sats_layout(&mut job, &Sats(input.value), &font_id);
                    newline(&mut job, &font_id);
                    address_layout(&mut job, &input.address, input.address_type, &font_id);
                    ui.label(job);
                });
            }

            // Calculate attraction force and update velocity
            let diff = to_rect.left_center() - from_rect.right_center();
            let mut force = diff.length_sq() / layout_params.scale * diff.normalized();
            force.y *= layout_params.y_compress;
            self.nodes.get_mut(&edge.source).unwrap().velocity += force * layout_params.dt;
            self.nodes.get_mut(&edge.target).unwrap().velocity -= force * layout_params.dt;

            // Repulsion force between layers
            let force = Vec2::new(scale2 / diff.x.max(2.0), 0.0);
            self.nodes.get_mut(&edge.source).unwrap().velocity -= force * layout_params.dt;
            self.nodes.get_mut(&edge.target).unwrap().velocity += force * layout_params.dt;
        }

        // Update positions
        for (_txid, node) in &mut self.nodes {
            node.velocity *= layout_params.cooloff;
            if !node.dragged {
                node.pos += node.velocity * layout_params.dt;
            }
        }
    }
}

fn clear_spacing(a: &Rect, b: &Rect) -> f32 {
    let x = (a.center().x - b.center().x).abs() - (b.width() + a.width()) / 2.0;
    let y = (a.center().y - b.center().y).abs() - (b.height() + a.height()) / 2.0;
    x.max(y).max(2.0)
}

pub fn rotated_layout(ui: &egui::Ui, job: LayoutJob, pos: Pos2, angle: f32) -> TextShape {
    let galley = ui.fonts(|f| f.layout_job(job));
    let mut shape = TextShape::new(pos, galley);
    shape.angle = angle;
    shape
}

fn tx_content(txid: &Txid, timestamp: &str, sats: &Sats) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font_id = FontId::monospace(10.0);
    txid_layout(&mut job, txid, &font_id);
    newline(&mut job, &font_id);
    sats_layout(&mut job, sats, &font_id);
    newline(&mut job, &font_id);
    job.append(
        &timestamp[2..],
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            color: Color32::BLACK,
            ..Default::default()
        },
    );
    job
}

fn newline(job: &mut LayoutJob, font_id: &FontId) {
    job.append(
        "\n",
        0.0,
        TextFormat {
            font_id: font_id.clone(),
            ..Default::default()
        },
    );
}

const SPACING: f32 = 3.0;

fn txid_layout(job: &mut LayoutJob, txid: &Txid, font_id: &FontId) {
    let black_format = TextFormat {
        font_id: font_id.clone(),
        color: Color32::BLACK,
        ..Default::default()
    };
    let white_format = TextFormat {
        font_id: font_id.clone(),
        color: Color32::from_gray(128),
        ..Default::default()
    };

    let mut first = true;
    let mut black = true;

    for chunk in txid.chunks() {
        job.append(
            &chunk,
            if first { 0.0 } else { SPACING },
            if black {
                black_format.clone()
            } else {
                white_format.clone()
            },
        );
        first = false;
        black = !black;
    }
}

fn sats_layout(job: &mut LayoutJob, sats: &Sats, font_id: &FontId) {
    let black_format = TextFormat {
        font_id: font_id.clone(),
        color: Color32::BLACK,
        ..Default::default()
    };
    let white_format = TextFormat {
        font_id: font_id.clone(),
        color: Color32::from_gray(128),
        ..Default::default()
    };

    let AmountComponents {
        sats,
        ksats,
        msats,
        btc,
    } = sats.components();

    let mut started = false;

    if btc.len() > 0 {
        job.append(&format!("{}", btc[0]), 0.0, black_format.clone());
        started = true;

        for amount in btc.iter().skip(1) {
            job.append(&format!("{:03}", amount), SPACING, black_format.clone());
        }
    } else {
        job.append("0", 0.0, white_format.clone());
    }

    job.append(
        ".",
        0.0,
        if started {
            black_format.clone()
        } else {
            white_format.clone()
        },
    );

    if started {
        job.append(
            &format!("{:02}", msats.unwrap_or(0)),
            0.0,
            black_format.clone(),
        );
    } else {
        if let Some(m) = msats {
            if m < 10 {
                job.append("0", 0.0, white_format.clone());
            }
            job.append(&format!("{}", m), 0.0, black_format.clone());
            started = true;
        } else {
            job.append("00", 0.0, white_format.clone());
        }
    }

    job.append("", SPACING, white_format.clone());
    if started {
        job.append(
            &format!("{:03}", ksats.unwrap_or(0)),
            0.0,
            black_format.clone(),
        );
    } else {
        if let Some(k) = ksats {
            if k < 10 {
                job.append("00", 0.0, white_format.clone());
            } else if k < 100 {
                job.append("0", 0.0, white_format.clone());
            }
            job.append(&format!("{}", k), 0.0, black_format.clone());
            started = true;
        } else {
            job.append("000", 0.0, white_format.clone());
        }
    }

    job.append("", SPACING, white_format.clone());
    if started {
        job.append(&format!("{:03}", sats), 0.0, black_format.clone());
    } else {
        if sats < 10 {
            job.append("00", 0.0, white_format.clone());
        } else if sats < 100 {
            job.append("0", 0.0, white_format.clone());
        }
        job.append(&format!("{}", sats), 0.0, black_format.clone());
    }

    job.append("sats", SPACING, black_format.clone());
}

fn address_layout(job: &mut LayoutJob, address: &str, address_type: AddressType, font_id: &FontId) {
    let black_format = TextFormat {
        font_id: font_id.clone(),
        color: Color32::BLACK,
        ..Default::default()
    };
    let white_format = TextFormat {
        color: Color32::from_gray(128),
        ..black_format.clone()
    };
    let highlight_format = TextFormat {
        color: Color32::BLUE,
        ..black_format.clone()
    };
    let mut small = font_id.clone();
    small.size *= 0.7;
    let type_format = TextFormat {
        font_id: small,
        valign: Align::Center,
        ..black_format.clone()
    };

    let highlight = match address_type {
        AddressType::P2TR => 4,   // "bc1p"
        AddressType::P2PKH => 1,  // "1"
        AddressType::P2SH => 1,   // "3"
        AddressType::P2WPKH => 4, // "bc1q"
        AddressType::P2WSH => 4,  // "bc1q"
    };

    job.append(&address[0..highlight], 0.0, highlight_format);
    job.append(&address[highlight..4], 0.0, black_format.clone());

    let mut black = false;
    for i in 1..=(address.len() / 4) {
        let from = i * 4;
        let to = (i * 4 + 4).min(address.len());
        job.append(
            &address[from..to],
            SPACING,
            if black {
                black_format.clone()
            } else {
                white_format.clone()
            },
        );
        black = !black;
    }

    let type_ = match address_type {
        AddressType::P2PKH => "p2pkh",
        AddressType::P2SH => "p2sh",
        AddressType::P2WPKH => "p2wpkh",
        AddressType::P2WSH => "p2wsh",
        AddressType::P2TR => "p2tr",
    };

    job.append(&format!(" ({})", type_), 0.0, type_format);
}
