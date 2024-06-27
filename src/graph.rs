use std::{collections::HashMap, fmt::Write, sync::mpsc::Sender};

use egui::{
    ahash::HashSet, text::LayoutJob, Align, Color32, CursorIcon, FontId, Mesh, Pos2, Rect,
    RichText, Rounding, Sense, Stroke, TextFormat, Vec2,
};
use serde::{Deserialize, Serialize};

use crate::{
    annotations::Annotations,
    app::Update,
    bezier::Edge,
    bitcoin::{AddressType, AmountComponents, Sats, SatsDisplay, Transaction, Txid},
    components::Components,
    export,
    force::ForceCalculator,
    layout::{Layout, Scale},
    platform::inner::push_history_state,
    style::{self, Style},
    transform::Transform,
};

#[derive(Serialize, Deserialize)]
pub struct Graph {
    nodes: HashMap<Txid, DrawableNode>,
    edges: Vec<DrawableEdge>,
    selected_node: Option<Txid>,
    components: Components,
}

#[derive(Serialize, Deserialize)]
pub struct DrawableNode {
    /// Center of tx rect.
    pos: Pos2,
    velocity: Vec2,
    dragged: bool,
    size: f32,
    tx_value: u64,
    tx_timestamp: String,
    block_height: u32,
    inputs: Vec<DrawableInput>,
    outputs: Vec<DrawableOutput>,
}

impl DrawableNode {
    fn scale(&mut self, scale: &Scale) {
        self.size = scale.apply(self.tx_value) as f32;

        let input_size: f32 = self
            .inputs
            .iter()
            .map(|input| scale.apply(input.value) as f32)
            .sum();

        let output_size: f32 = self
            .outputs
            .iter()
            .map(|output| scale.apply(output.value) as f32)
            .sum();

        let mut end = 0.0;

        for i in &mut self.inputs {
            let h = scale.apply(i.value) as f32 * self.size / input_size;
            i.start = end;
            end += h;
            i.end = end;
        }

        end = 0.0;

        for o in &mut self.outputs {
            let h = scale.apply(o.value) as f32 * self.size / output_size;
            o.start = end;
            end += h;
            o.end = end;
        }
    }

    fn export_beancount(&self, txid: &Txid, label: Option<String>) -> String {
        let mut s = String::new();
        writeln!(
            s,
            "{} * \"{}\" ^{}",
            &self.tx_timestamp[0..10],
            label.unwrap_or("".to_string()),
            txid.hex_string()
        )
        .unwrap();
        for input in &self.inputs {
            writeln!(
                s,
                "  Assets:Bitcoin:{:<72} {:>20.8} BTC",
                input.address,
                -(input.value as f64) / 100_000_000.0
            )
            .unwrap();
        }
        for output in &self.outputs {
            let account = match &output.output_type {
                OutputType::Fees => format!("Expenses:Bitcoin:Fees{:66}", " "),
                OutputType::Spent {
                    spending_txid: _,
                    address,
                    address_type: _,
                } => format!("Assets:Bitcoin:{:<72}", address),
                OutputType::Utxo {
                    address,
                    address_type: _,
                } => format!("Assets:Bitcoin:{:<72}", address),
            };
            if output.value > 0 {
                writeln!(
                    s,
                    "  {} {:>20.8} BTC",
                    account,
                    (output.value as f64) / 100_000_000.0
                )
                .unwrap();
            }
        }
        s
    }
}

#[derive(Clone, Hash, Serialize, Deserialize)]
pub struct DrawableEdge {
    source: Txid,
    source_pos: usize,
    target: Txid,
    target_pos: usize,
}

#[derive(Serialize, Deserialize)]
pub struct DrawableInput {
    start: f32,
    end: f32,
    value: u64,
    address: String,
    address_type: AddressType,
    funding_txid: Txid, // TODO: coinbase tx?
    funding_vout: u32,
}

#[derive(Serialize, Deserialize)]
pub struct DrawableOutput {
    start: f32,
    end: f32,
    value: u64,
    output_type: OutputType,
}

#[derive(Serialize, Deserialize)]
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

impl Default for Graph {
    fn default() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
            selected_node: None,
            components: Components::new(),
        }
    }
}

impl Graph {
    pub fn export(&self) -> Vec<export::Transaction> {
        self.nodes
            .iter()
            .map(|(txid, node)| export::Transaction::new(*txid, node.pos))
            .collect()
    }

    fn add_edge(&mut self, edge: DrawableEdge) {
        self.components.connect(edge.source, edge.target);
        self.edges.push(edge);
    }

    pub fn get_tx_pos(&self, txid: Txid) -> Option<Pos2> {
        self.nodes.get(&txid).map(|node| node.pos)
    }

    pub fn select(&mut self, txid: Txid) {
        self.selected_node = Some(txid);
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

        let inputs = tx
            .inputs
            .iter()
            .map(|i| DrawableInput {
                start: 0.0,
                end: 0.0,
                value: i.value,
                address: i.address.clone(),
                address_type: i.address_type,
                funding_txid: i.txid,
                funding_vout: i.vout,
            })
            .collect();

        let mut outputs: Vec<DrawableOutput> = tx
            .outputs
            .iter()
            .map(|o| DrawableOutput {
                start: 0.0,
                end: 0.0,
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
            })
            .collect();

        // Coinbase txs don't have fees
        if !tx.is_coinbase() {
            outputs.push(DrawableOutput {
                start: 0.0,
                end: 0.0,
                value: tx.fees(),
                output_type: OutputType::Fees,
            });
        }

        self.nodes.insert(
            txid,
            DrawableNode {
                pos,
                velocity: Vec2::new(0.0, 0.0),
                dragged: false,
                size: 0.0,
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
        update_sender: Sender<Update>,
        layout: &Layout,
        annotations: &mut Annotations,
        loading_txids: &HashSet<Txid>,
        force_calculator: &ForceCalculator,
    ) {
        let style = style::get(ui);

        let clip_rect = ui.clip_rect();

        for node in self.nodes.values_mut() {
            node.scale(&layout.scale);
        }

        // PREPARE RECTS //

        let mut input_rects: HashMap<(Txid, usize), Rect> = HashMap::new();
        let mut output_rects: HashMap<(Txid, usize), Rect> = HashMap::new();
        let mut inner_rects: HashMap<Txid, Rect> = HashMap::new();
        let mut outer_rects: HashMap<Txid, Rect> = HashMap::new();

        for (txid, node) in &self.nodes {
            let outer_rect = Rect::from_center_size(
                node.pos,
                Vec2::new(node.size, style.tx_width + 2.0 * style.io_width),
            );
            let inner_rect = Rect::from_center_size(node.pos, Vec2::new(node.size, style.tx_width));

            outer_rects.insert(*txid, outer_rect);
            inner_rects.insert(*txid, inner_rect);

            let left_top = outer_rect.left_top();
            for (i, input) in node.inputs.iter().enumerate() {
                let rect = Rect::from_min_max(
                    Pos2::new(left_top.x + input.start, left_top.y),
                    Pos2::new(left_top.x + input.end, left_top.y + style.io_width),
                );
                input_rects.insert((*txid, i), rect);
            }

            let left_bot = outer_rect.left_bottom();
            for (o, output) in node.outputs.iter().enumerate() {
                let rect = Rect::from_min_max(
                    Pos2::new(left_bot.x + output.start, left_bot.y - style.io_width),
                    Pos2::new(left_bot.x + output.end, left_bot.y),
                );
                output_rects.insert((*txid, o), rect);
            }
        }

        // DRAW EDGES //

        for edge in &self.edges {
            let from_rect = output_rects.get(&(edge.source, edge.source_pos)).unwrap();
            let to_rect = input_rects.get(&(edge.target, edge.target_pos)).unwrap();

            let bounding_rect = transform.rect_to_screen(from_rect.union(*to_rect));
            if !clip_rect.intersects(bounding_rect) {
                continue;
            }

            let coin = (edge.source, edge.source_pos);
            let color = annotations.coin_color(coin).unwrap_or(Color32::GOLD);

            let flow = Edge {
                from: from_rect.left_bottom(),
                from_width: from_rect.width(),
                to: to_rect.left_top(),
                to_width: to_rect.width(),
            };

            let response = flow
                .draw(ui, color, layout.show_arrows, transform, &coin)
                .on_hover_ui_at_pointer(|ui| {
                    if let Some(label) = annotations.coin_label(coin) {
                        ui.label(RichText::new(format!("[{}]", label)).heading().monospace());
                    }
                    let input = &self.nodes.get(&edge.target).unwrap().inputs[edge.target_pos];
                    let mut job = LayoutJob::default();
                    sats_layout(&mut job, &Sats(input.value), &style);
                    newline(&mut job, &style.font_id());
                    address_layout(&mut job, &input.address, input.address_type, &style);
                    ui.label(job);
                });
            response.context_menu(|ui| annotations.coin_menu(coin, ui));

            if response.clicked {
                ui.output_mut(|o| {
                    o.copied_text = self.nodes.get(&edge.target).unwrap().inputs[edge.target_pos]
                        .address
                        .clone()
                });
            }
        }

        // DRAW NODES //

        let initial_dist = Vec2::new(0.0, style.io_width + style.tx_width / 2.0 + 5.0);
        let painter = ui.painter();
        let txids: HashSet<Txid> = self.nodes.keys().copied().collect();

        for (txid, node) in &mut self.nodes {
            let outer_rect = transform.rect_to_screen(*outer_rects.get(txid).unwrap());

            if !clip_rect.intersects(outer_rect.expand(style.selected_stroke_width * 2.0)) {
                continue;
            }

            if Some(*txid) == self.selected_node {
                painter.rect(
                    outer_rect.expand(style.selected_stroke_width / 2.0),
                    Rounding::ZERO,
                    Color32::TRANSPARENT,
                    style.selected_tx_stroke(),
                );
            }

            let label = annotations.tx_label(*txid);
            let rect = transform.rect_to_screen(*inner_rects.get(txid).unwrap());
            let response = ui
                .interact(rect, ui.id().with(txid), Sense::click_and_drag())
                .on_hover_ui(|ui| {
                    let format = TextFormat {
                        font_id: style.font_id(),
                        color: style.black_text_color(),
                        ..Default::default()
                    };

                    ui.label(RichText::new("Transaction").heading().monospace());
                    let mut job = LayoutJob::default();
                    txid_layout(&mut job, txid, &style);
                    newline(&mut job, &style.font_id());
                    if let Some(label) = label.clone() {
                        job.append(&format!("[{}]", label), 0.0, format.clone());
                        newline(&mut job, &style.font_id());
                    }
                    newline(&mut job, &FontId::monospace(5.0));
                    sats_layout(&mut job, &Sats(node.tx_value), &style);
                    job.append(
                        &format!("\n{} (block {})", node.tx_timestamp, node.block_height),
                        0.0,
                        format.clone(),
                    );
                    ui.label(job);
                });
            response.context_menu(|ui| {
                ui.menu_button("Annotate", |ui| annotations.tx_menu(*txid, ui));
                ui.menu_button("Export to Clipboard", |ui| {
                    if ui.button("Beancount").clicked() {
                        ui.ctx().output_mut(|o| {
                            o.copied_text = node.export_beancount(txid, label.clone())
                        });
                        ui.close_menu();
                    }
                });
                if ui.button("Copy Txid").clicked() {
                    ui.output_mut(|o| o.copied_text = txid.hex_string());
                    ui.close_menu();
                }
                if ui.button("Remove").clicked() {
                    update_sender
                        .send(Update::RemoveTx { txid: *txid })
                        .unwrap();
                    ui.close_menu();
                }
            });

            if response.clicked() {
                push_history_state(&format!("tx/{}", txid.hex_string()));
                update_sender
                    .send(Update::SelectTx { txid: *txid })
                    .unwrap();
            }

            if response.hovered() {
                ui.output_mut(|o| o.cursor_icon = CursorIcon::Grab);
            }

            if response.dragged() {
                node.dragged = true;
                node.velocity = Vec2::ZERO;
                node.pos += transform.vec_from_screen(response.drag_delta());
                ui.output_mut(|o| o.cursor_icon = CursorIcon::Grabbing);
            } else {
                node.dragged = false;
            }

            painter.rect(
                rect,
                Rounding::ZERO,
                annotations
                    .tx_color(*txid)
                    .unwrap_or(style.tx_bg)
                    .gamma_multiply(0.4),
                style.tx_stroke(),
            );

            let tx_painter = painter.with_clip_rect(rect);
            tx_painter.galley(
                rect.left_top() + Vec2::new(2.0, 2.0),
                tx_painter.layout_job(tx_content(
                    txid,
                    &label,
                    &node.tx_timestamp,
                    &Sats(node.tx_value),
                    &style,
                )),
                Color32::TRANSPARENT,
            );

            let id = ui.id().with("i").with(txid);
            for (i, input) in node.inputs.iter().enumerate() {
                let coin = (input.funding_txid, input.funding_vout as usize);

                let rect = *input_rects.get(&(*txid, i)).unwrap();
                let screen_rect = transform.rect_to_screen(rect);
                let response = ui
                    .interact(screen_rect, id.with(i), Sense::click())
                    .on_hover_ui(|ui| {
                        let label = match annotations.coin_label(coin) {
                            Some(l) => format!(" [{}]", l),
                            None => "".to_string(),
                        };
                        ui.label(
                            RichText::new(format!("⏴Input{}", label))
                                .heading()
                                .monospace(),
                        );
                        let mut job = LayoutJob::default();
                        sats_layout(&mut job, &Sats(input.value), &style);
                        newline(&mut job, &style.font_id());
                        address_layout(&mut job, &input.address, input.address_type, &style);
                        newline(&mut job, &style.font_id());
                        newline(&mut job, &FontId::monospace(5.0));
                        txid_layout(&mut job, &input.funding_txid, &style);
                        ui.label(job);
                    });
                response.context_menu(|ui| annotations.coin_menu(coin, ui));

                if response.clicked() {
                    if txids.contains(&input.funding_txid) {
                        update_sender
                            .send(Update::RemoveTx {
                                txid: input.funding_txid,
                            })
                            .unwrap();
                    } else {
                        update_sender
                            .send(Update::LoadOrSelectTx {
                                txid: input.funding_txid,
                                pos: Some(rect.center_top() - initial_dist),
                            })
                            .unwrap();
                    }
                }

                painter.rect(
                    screen_rect,
                    Rounding::ZERO,
                    annotations
                        .coin_color(coin)
                        .unwrap_or(style.io_bg)
                        .gamma_multiply(0.4),
                    Stroke::NONE,
                );

                if loading_txids.contains(&input.funding_txid) {
                    rect_striped(
                        ui,
                        screen_rect,
                        style.black_text_color().gamma_multiply(0.1),
                    );
                    ui.ctx().request_repaint();
                }

                painter.rect(
                    screen_rect,
                    Rounding::ZERO,
                    Color32::TRANSPARENT,
                    style.io_stroke(&response),
                );
            }

            let id = ui.id().with("o").with(txid);
            // rev() so that we paint the fees first and they get overdrawn by the
            // hover boxes of the outpus.
            for (o, output) in node.outputs.iter().enumerate().rev() {
                let coin = (*txid, o);

                let rect = *output_rects.get(&(*txid, o)).unwrap();
                let screen_rect = transform.rect_to_screen(rect);
                let response = ui
                    .interact(screen_rect, id.with(o), Sense::click())
                    .on_hover_ui(|ui| match &output.output_type {
                        OutputType::Utxo {
                            address,
                            address_type,
                        } => {
                            let label = match annotations.coin_label(coin) {
                                Some(l) => format!(" [{}]", l),
                                None => "".to_string(),
                            };
                            ui.label(
                                RichText::new(format!("Unspent Output{}", label))
                                    .heading()
                                    .monospace(),
                            );
                            let mut job = LayoutJob::default();
                            sats_layout(&mut job, &Sats(output.value), &style);
                            newline(&mut job, &style.font_id());
                            address_layout(&mut job, address, *address_type, &style);
                            ui.label(job);
                        }
                        OutputType::Spent {
                            spending_txid,
                            address,
                            address_type,
                        } => {
                            let label = match annotations.coin_label(coin) {
                                Some(l) => format!(" [{}]", l),
                                None => "".to_string(),
                            };
                            ui.label(
                                RichText::new(format!("Output⏵{}", label))
                                    .heading()
                                    .monospace(),
                            );
                            let mut job = LayoutJob::default();
                            sats_layout(&mut job, &Sats(output.value), &style);
                            newline(&mut job, &style.font_id());
                            address_layout(&mut job, address, *address_type, &style);
                            newline(&mut job, &style.font_id());
                            newline(&mut job, &FontId::monospace(5.0));
                            txid_layout(&mut job, spending_txid, &style);
                            ui.label(job);
                        }
                        OutputType::Fees => {
                            ui.label(RichText::new("Fees").heading().monospace());
                            ui.add(SatsDisplay::new(Sats(output.value), &style));
                        }
                    });

                match output.output_type {
                    OutputType::Fees => {}
                    _ => {
                        response.context_menu(|ui| annotations.coin_menu(coin, ui));
                    }
                }

                if let OutputType::Spent {
                    spending_txid,
                    address: _,
                    address_type: _,
                } = &output.output_type
                {
                    if response.clicked() {
                        if txids.contains(spending_txid) {
                            update_sender
                                .send(Update::RemoveTx {
                                    txid: *spending_txid,
                                })
                                .unwrap();
                        } else {
                            update_sender
                                .send(Update::LoadOrSelectTx {
                                    txid: *spending_txid,
                                    pos: Some(rect.center_bottom() + initial_dist),
                                })
                                .unwrap();
                        }
                    }
                }

                painter.rect(
                    screen_rect,
                    Rounding::ZERO,
                    match output.output_type {
                        OutputType::Utxo {
                            address: _,
                            address_type: _,
                        } => annotations
                            .coin_color(coin)
                            .unwrap_or(style.utxo_fill())
                            .gamma_multiply(0.4),
                        OutputType::Spent {
                            spending_txid: _,
                            address: _,
                            address_type: _,
                        } => annotations
                            .coin_color(coin)
                            .unwrap_or(style.io_bg)
                            .gamma_multiply(0.4),
                        OutputType::Fees => style.fees_fill(),
                    },
                    Stroke::NONE,
                );

                if let OutputType::Spent { spending_txid, .. } = output.output_type {
                    if loading_txids.contains(&spending_txid) {
                        rect_striped(
                            ui,
                            screen_rect,
                            style.black_text_color().gamma_multiply(0.1),
                        );
                        ui.ctx().request_repaint();
                    }
                }

                painter.rect(
                    screen_rect,
                    Rounding::ZERO,
                    Color32::TRANSPARENT,
                    match output.output_type {
                        OutputType::Spent {
                            spending_txid: _,
                            address: _,
                            address_type: _,
                        } => style.io_stroke(&response),
                        _ => style.tx_stroke(),
                    },
                );
            }
        }

        // CALCULATE FORCES AND UPDATE VELOCITY //

        let scale2 = layout.force_params.scale * layout.force_params.scale;

        let (txids, rects): (Vec<Txid>, Vec<Rect>) = outer_rects.iter().unzip();
        let forces = force_calculator.calculate_forces(
            layout.force_params.scale,
            layout.force_params.tx_repulsion_radius,
            &rects,
        );
        for (txid, force) in txids.iter().zip(forces) {
            self.nodes.get_mut(txid).unwrap().velocity += force * layout.force_params.dt;
        }

        // Calculate edge multiplicities to deal with transactions sharing
        // multiple inputs/outputs.
        let mut edge_multiplicities: HashMap<(Txid, Txid), usize> = HashMap::new();
        for edge in &self.edges {
            let key = (edge.source, edge.target);
            *edge_multiplicities.entry(key).or_insert(0) += 1;
        }

        for edge in &self.edges {
            let from_rect = output_rects.get(&(edge.source, edge.source_pos)).unwrap();
            let to_rect = input_rects.get(&(edge.target, edge.target_pos)).unwrap();

            // Attraction force between nodes
            let diff = to_rect.center_top() - from_rect.center_bottom();
            let mut force = diff.length_sq() / layout.force_params.scale * diff.normalized();

            // Repulsion force between layers
            force -= Vec2::new(0.0, scale2 / diff.y.max(2.0));

            // Take edge multiplicity into account
            force /= edge_multiplicities[&(edge.source, edge.target)] as f32;

            self.nodes.get_mut(&edge.source).unwrap().velocity += force * layout.force_params.dt;
            self.nodes.get_mut(&edge.target).unwrap().velocity -= force * layout.force_params.dt;
        }

        // UPDATE POSITIONS //

        for node in self.nodes.values_mut() {
            node.velocity *= layout.force_params.cooloff;
            if node.velocity.length() > 0.2 {
                ui.ctx().request_repaint();
            }
            if !node.dragged {
                node.pos += node.velocity * layout.force_params.dt;
            }
        }
    }
}

fn tx_content(
    txid: &Txid,
    label: &Option<String>,
    timestamp: &str,
    sats: &Sats,
    style: &Style,
) -> LayoutJob {
    let mut job = LayoutJob::default();
    let font_id = FontId::monospace(10.0);
    let format = TextFormat {
        font_id: font_id.clone(),
        color: style.black_text_color(),
        ..Default::default()
    };

    if let Some(label) = label {
        job.append(label, 0.0, format.clone());
    } else {
        txid_layout(&mut job, txid, style);
    }
    newline(&mut job, &font_id);
    sats_layout(&mut job, sats, style);
    newline(&mut job, &font_id);
    job.append(&timestamp[2..], 0.0, format);
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

fn txid_layout(job: &mut LayoutJob, txid: &Txid, style: &Style) {
    let black_format = TextFormat {
        font_id: style.font_id(),
        color: style.black_text_color(),
        ..Default::default()
    };
    let white_format = TextFormat {
        font_id: style.font_id(),
        color: style.white_text_color(),
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

pub fn sats_layout(job: &mut LayoutJob, sats: &Sats, style: &Style) {
    let font_id = style.font_id();
    let btc_font = FontId::new(font_id.size, egui::FontFamily::Name("btc".into()));
    let btc_format = TextFormat {
        font_id: btc_font,
        color: style.btc,
        ..Default::default()
    };
    job.append("\u{E9A8}", 0.0, btc_format);

    let amount = sats.0;

    let black_format = TextFormat {
        font_id: font_id.clone(),
        color: style.black_text_color(),
        ..Default::default()
    };
    let white_format = TextFormat {
        font_id: font_id.clone(),
        color: style.white_text_color(),
        ..Default::default()
    };

    let AmountComponents {
        sats,
        ksats,
        msats,
        btc,
    } = sats.components();

    let mut started = false;

    if !btc.is_empty() {
        job.append(&format!("{}", btc[0]), SPACING, black_format.clone());
        started = true;

        for amount in btc.iter().skip(1) {
            job.append(&format!("{:03}", amount), SPACING, black_format.clone());
        }
    } else {
        job.append(
            "0",
            SPACING,
            if amount % 1_000_000 == 0 && amount > 100_000 {
                black_format.clone()
            } else {
                white_format.clone()
            },
        );
    }

    #[allow(clippy::collapsible_else_if)]
    job.append(
        ".",
        0.0,
        if started {
            if amount % 100_000_000 == 0 {
                white_format.clone()
            } else {
                black_format.clone()
            }
        } else {
            if amount % 1_000_000 == 0 && amount > 100_000 {
                black_format.clone()
            } else {
                white_format.clone()
            }
        },
    );

    if started {
        job.append(
            &format!("{:02}", msats.unwrap_or(0)),
            0.0,
            if amount % 100_000_000 == 0 {
                white_format.clone()
            } else {
                black_format.clone()
            },
        );
    } else if let Some(m) = msats {
        if m < 10 {
            job.append(
                "0",
                0.0,
                if amount % 1_000_000 == 0 {
                    black_format.clone()
                } else {
                    white_format.clone()
                },
            );
        }
        job.append(&format!("{}", m), 0.0, black_format.clone());
        started = true;
    } else {
        job.append("00", 0.0, white_format.clone());
    }

    job.append("", SPACING, white_format.clone());
    if started {
        job.append(
            &format!("{:03}", ksats.unwrap_or(0)),
            0.0,
            if amount % 1_000_000 == 0 {
                white_format.clone()
            } else {
                black_format.clone()
            },
        );
    } else if let Some(k) = ksats {
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

    job.append("", SPACING, white_format.clone());
    if started {
        job.append(
            &format!("{:03}", sats),
            0.0,
            if amount % 1_000_000 == 0 {
                white_format.clone()
            } else {
                black_format.clone()
            },
        );
    } else {
        if sats < 10 {
            job.append("00", 0.0, white_format);
        } else if sats < 100 {
            job.append("0", 0.0, white_format);
        }
        job.append(&format!("{}", sats), 0.0, black_format.clone());
    }

    job.append("sats", SPACING, black_format);
}

fn address_layout(job: &mut LayoutJob, address: &str, address_type: AddressType, style: &Style) {
    let black_format = TextFormat {
        font_id: style.font_id(),
        color: style.black_text_color(),
        ..Default::default()
    };
    let white_format = TextFormat {
        color: style.white_text_color(),
        ..black_format.clone()
    };
    let highlight_format = TextFormat {
        color: style.tx_bg,
        ..black_format.clone()
    };
    let mut small = style.font_id();
    small.size *= 0.7;
    let type_format = TextFormat {
        font_id: small,
        valign: Align::Center,
        ..black_format
    };

    let highlight = match address_type {
        AddressType::P2TR => 4,   // "bc1p"
        AddressType::P2PKH => 1,  // "1"
        AddressType::P2SH => 1,   // "3"
        AddressType::P2WPKH => 4, // "bc1q"
        AddressType::P2WSH => 4,  // "bc1q"
        AddressType::Unknown => 0,
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
        AddressType::Unknown => "?",
    };

    job.append(&format!(" ({})", type_), 0.0, type_format);
}

/// Fill the given rect with an animated striped pattern.
fn rect_striped(ui: &egui::Ui, rect: Rect, color: Color32) {
    let width: f32 = 6.;
    let speed = 20.;
    let time = ui.input(|i| i.time);

    let mut mesh = Mesh::default();
    let mut start = rect.left_top() + Vec2::new((time as f32 * speed % width * 2.) - width, 0.);
    let mut i0 = 0;

    loop {
        let nw = start;
        let sw = nw + Vec2::new(-rect.height(), rect.height());
        let ne = nw + Vec2::new(width, 0.);
        let se = sw + Vec2::new(width, 0.);

        mesh.colored_vertex(nw, color);
        mesh.colored_vertex(ne, color);
        mesh.colored_vertex(se, color);
        mesh.colored_vertex(sw, color);
        mesh.add_triangle(i0, i0 + 1, i0 + 2);
        mesh.add_triangle(i0, i0 + 2, i0 + 3);

        start += Vec2::new(width * 2.0, 0.);
        i0 += 4;

        if start.x > rect.right_top().x + rect.height() + width {
            break;
        }
    }

    ui.painter().with_clip_rect(rect).add(mesh);
}
