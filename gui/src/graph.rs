use std::collections::HashMap;

use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

use crate::{
    bezier::Edge,
    bitcoin::{Sats, Transaction, Txid},
    transform::Transform,
};

pub fn to_drawable(txs: &HashMap<Txid, Transaction>) -> DrawableGraph {
    let mut layers: Vec<Vec<Txid>> = Vec::new();

    let edges: Vec<DrawableEdge> = txs
        .iter()
        .flat_map(|(txid, tx)| {
            tx.inputs
                .iter()
                .enumerate()
                .filter_map(|(i, input)| {
                    txs.get(&input.txid).map(|input_tx| {
                        let o = input_tx
                            .outputs
                            .iter()
                            .enumerate()
                            .find(|(_, output)| output.spending_txid == Some(*txid))
                            .unwrap()
                            .0;
                        DrawableEdge {
                            source: input.txid,
                            source_pos: o,
                            target: *txid,
                            target_pos: i,
                        }
                    })
                })
                .collect::<Vec<DrawableEdge>>()
        })
        .collect();

    let mut no_inputs: HashMap<Txid, usize> = txs.iter().map(|(txid, _)| (*txid, 0)).collect();
    for edge in &edges {
        no_inputs
            .entry(edge.target)
            .and_modify(|n| *n += 1)
            .or_insert(0);
    }

    let mut next_layer: Vec<Txid> = no_inputs
        .iter()
        .filter_map(|(txid, n)| if *n == 0 { Some(*txid) } else { None })
        .collect();
    next_layer.sort();

    while !next_layer.is_empty() {
        let current_layer = next_layer.clone();
        layers.push(next_layer);
        next_layer = Vec::new();

        for txid in current_layer {
            for o in &txs.get(&txid).unwrap().outputs {
                if let Some(target) = o.spending_txid {
                    no_inputs.entry(target).and_modify(|n| {
                        *n -= 1;
                        if *n == 0 {
                            next_layer.push(target);
                        }
                    });
                }
            }
        }
    }

    fn scale(value: u64) -> f32 {
        f32::powf(value as f32, 1.0 / 3.0).round() / 10.0
    }

    let mut nodes = HashMap::new();

    let mut x = 0.0;
    const NODE_SEPARATION: f32 = 20.0;
    const LAYER_SEPARATION: f32 = 100.0;

    for layer in layers {
        let mut layer_height = -NODE_SEPARATION;
        for txid in &layer {
            layer_height += scale(txs.get(txid).unwrap().amount()) + NODE_SEPARATION;
        }

        let mut y = -layer_height / 2.0;

        for txid in &layer {
            let tx = txs.get(txid).unwrap();
            let height = scale(tx.amount());

            y += height / 2.0;

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
                        address_type: i.address_type.clone(),
                        funding_txid: i.txid,
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
                                address_type: o.address_type.clone(),
                            },
                            Some(txid) => OutputType::Spent {
                                spending_txid: txid,
                                address: o.address.clone(),
                                address_type: o.address_type.clone(),
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

            nodes.insert(
                *txid,
                DrawableNode {
                    pos: Pos2::new(x, y),
                    velocity: Vec2::new(0.0, 0.0),
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

            y += height / 2.0 + NODE_SEPARATION;
        }

        x += LAYER_SEPARATION;
    }

    DrawableGraph { nodes, edges }
}

pub struct DrawableGraph {
    nodes: HashMap<Txid, DrawableNode>,
    edges: Vec<DrawableEdge>,
}

pub struct DrawableNode {
    /// Center of tx rect.
    pos: Pos2,
    velocity: Vec2,
    height: f32,
    tx_value: u64,
    tx_timestamp: String,
    block_height: u32,
    inputs: Vec<DrawableInput>,
    outputs: Vec<DrawableOutput>,
}

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
    address_type: String,
    funding_txid: Txid, // TODO: coinbase tx?
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
        address_type: String,
    },
    Spent {
        spending_txid: Txid,
        address: String,
        address_type: String,
    },
    Fees,
}

impl DrawableGraph {
    pub fn empty() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: Vec::new(),
        }
    }

    pub fn draw(&mut self, ui: &egui::Ui, transform: &Transform, click_tx: impl Fn(Txid)) {
        const TX_WIDTH: f32 = 20.0;
        const IO_WIDTH: f32 = 10.0;

        const DT: f32 = 0.02;
        const SCALE: f32 = 5.0;
        const COOLOFF: f32 = 0.90;

        let painter = ui.painter();

        let mut input_rects: HashMap<(Txid, usize), Rect> = HashMap::new();
        let mut output_rects: HashMap<(Txid, usize), Rect> = HashMap::new();

        let positions: HashMap<Txid, Pos2> = self.nodes.iter().map(|(t, n)| (*t, n.pos)).collect();

        for (txid, node) in &mut self.nodes {
            let top_left = node.pos + Vec2::new(TX_WIDTH / 2.0, -node.height / 2.0);
            let rect = transform.rect_to_screen(Rect::from_min_size(
                top_left,
                Vec2::new(TX_WIDTH, node.height),
            ));
            let _response = ui
                .interact(rect, ui.id().with(txid), Sense::hover())
                .on_hover_ui(|ui| {
                    ui.label(format!("Tx: {}", txid));
                    ui.label(format!("Total amount: {}", Sats(node.tx_value)));
                    ui.label(format!(
                        "Timestamp: {}, block: {}",
                        node.tx_timestamp, node.block_height
                    ));
                });
            painter.rect(
                rect,
                Rounding::none(),
                Color32::LIGHT_RED,
                Stroke::new(1.0, Color32::BLACK),
            );

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
                        ui.label(format!("{} sats", Sats(input.value)));
                        ui.label(format!(
                            "Address: {} ({})",
                            input.address, input.address_type
                        ));
                        ui.label(format!("Previous Tx: {}", input.funding_txid));
                    });

                if response.clicked() {
                    click_tx(input.funding_txid);
                }

                painter.rect_stroke(
                    screen_rect,
                    Rounding::none(),
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
                    .on_hover_ui(|ui| {
                        ui.label(format!("{} sats", Sats(output.value)));
                        match &output.output_type {
                            OutputType::Utxo {
                                address,
                                address_type,
                            } => {
                                ui.label(format!("Address: {} ({})", address, address_type));
                                ui.label("UTXO!".to_string());
                            }
                            OutputType::Spent {
                                spending_txid,
                                address,
                                address_type,
                            } => {
                                ui.label(format!("Address: {} ({})", address, address_type));
                                ui.label(format!("Spending Tx: {}", spending_txid));
                            }
                            OutputType::Fees => {
                                ui.label("Fees!".to_string());
                            }
                        }
                    });

                if let OutputType::Spent {
                    spending_txid,
                    address: _,
                    address_type: _,
                } = &output.output_type
                {
                    if response.clicked() {
                        click_tx(*spending_txid);
                    }
                }

                painter.rect(
                    screen_rect,
                    Rounding::none(),
                    match output.output_type {
                        OutputType::Utxo {
                            address: _,
                            address_type: _,
                        } => Color32::GRAY,
                        OutputType::Spent {
                            spending_txid: _,
                            address: _,
                            address_type: _,
                        } => Color32::TRANSPARENT,
                        OutputType::Fees => Color32::BLACK,
                    },
                    ui.style().interact(&response).fg_stroke,
                );

                output_rects.insert((*txid, o), rect);
            }

            // Calculate repulsion force and update velocity;
            for (other_txid, other_node_pos) in &positions {
                if *other_txid == *txid {
                    continue;
                }
                let diff = *other_node_pos - node.pos;
                let force = -(SCALE * SCALE) * diff.length() * diff.normalized();
                node.velocity += force * DT;
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
            flow.draw(&ui, &transform);

            // Calculate attraction force and update velocity
            let diff = to_rect.left_center() - from_rect.right_center();
            let force = diff.length_sq() / SCALE * diff.normalized();
            self.nodes.get_mut(&edge.source).unwrap().velocity += force * DT;
            self.nodes.get_mut(&edge.target).unwrap().velocity -= force * DT;
        }

        // Update positions
        for (_txid, node) in &mut self.nodes {
            node.velocity *= COOLOFF;
            node.pos += node.velocity * DT;
        }
    }
}
