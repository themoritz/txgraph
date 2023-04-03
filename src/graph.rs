use std::collections::HashMap;

use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use electrum_client::bitcoin::{Script, Txid};

use crate::{
    bezier::Edge,
    bitcoin::{script_to_address, Sats, Transaction},
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
                            .find(|(_, output)| output.spend_txid == Some(*txid))
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
                if let Some(target) = o.spend_txid {
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

    let mut x = 100.0;
    const NODE_SEPARATION: f32 = 20.0;

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
                        script: i.script.clone(),
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
                        output_type: match o.spend_txid {
                            None => OutputType::Utxo {
                                script: o.script.clone(),
                            },
                            Some(txid) => OutputType::Spent {
                                spending_txid: txid,
                                script: o.script.clone(),
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
                    height,
                    inputs,
                    outputs,
                },
            );

            y += height / 2.0 + NODE_SEPARATION;
        }

        x += 100.0;
    }

    DrawableGraph { nodes, edges }
}

pub struct DrawableGraph {
    nodes: HashMap<Txid, DrawableNode>,
    edges: Vec<DrawableEdge>,
}

pub struct DrawableNode {
    pos: Pos2,
    height: f32,
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
    script: Script,
    funding_txid: Txid, // TODO: coinbase tx?
}

pub struct DrawableOutput {
    top: f32,
    bot: f32,
    value: u64,
    output_type: OutputType,
}

pub enum OutputType {
    Utxo { script: Script },
    Spent { spending_txid: Txid, script: Script },
    Fees,
}

impl DrawableGraph {
    pub fn draw(&self, ui: &egui::Ui, transform: &Transform, mut click_tx: impl FnMut(Txid)) {
        let painter = ui.painter();

        let mut input_rects: HashMap<(Txid, usize), Rect> = HashMap::new();
        let mut output_rects: HashMap<(Txid, usize), Rect> = HashMap::new();

        for (txid, node) in &self.nodes {
            let top_left = node.pos + Vec2::new(-5.0, -node.height / 2.0);
            let rect = transform
                .rect_to_screen(Rect::from_min_size(top_left, Vec2::new(10.0, node.height)));
            painter.rect(
                rect,
                Rounding::none(),
                Color32::LIGHT_RED,
                Stroke::new(1.0, Color32::BLACK),
            );

            let id = ui.id().with("i").with(txid);
            for (i, input) in node.inputs.iter().enumerate() {
                let rect = Rect::from_min_max(
                    Pos2::new(top_left.x - 10.0, top_left.y + input.top),
                    Pos2::new(top_left.x, top_left.y + input.bot),
                );
                let screen_rect = transform.rect_to_screen(rect);
                let response = ui
                    .interact(screen_rect, id.with(i), Sense::click())
                    .on_hover_ui(|ui| {
                        ui.label(format!("{} sats", Sats(input.value)));
                        ui.label(format!("Address: {}", script_to_address(&input.script)));
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
                    Pos2::new(top_left.x + 10.0, top_left.y + output.top),
                    Pos2::new(top_left.x + 20.0, top_left.y + output.bot),
                );
                let screen_rect = transform.rect_to_screen(rect);
                let response = ui
                    .interact(screen_rect, id.with(o), Sense::click())
                    .on_hover_ui(|ui| {
                        ui.label(format!("{} sats", Sats(output.value)));
                        match &output.output_type {
                            OutputType::Utxo { script } => {
                                ui.label(format!("Address: {}", script_to_address(script)));
                                ui.label("UTXO!".to_string());
                            }
                            OutputType::Spent {
                                spending_txid,
                                script,
                            } => {
                                ui.label(format!("Address: {}", script_to_address(script)));
                                ui.label(format!("Spending Tx: {}", spending_txid));
                            }
                            OutputType::Fees => {
                                ui.label("Fees!".to_string());
                            }
                        }
                    });

                if let OutputType::Spent {
                    spending_txid,
                    script: _,
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
                        OutputType::Utxo { script: _ } => Color32::GRAY,
                        OutputType::Spent {
                            spending_txid: _,
                            script: _,
                        } => Color32::TRANSPARENT,
                        OutputType::Fees => Color32::BLACK,
                    },
                    ui.style().interact(&response).fg_stroke,
                );

                output_rects.insert((*txid, o), rect);
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
        }
    }
}
