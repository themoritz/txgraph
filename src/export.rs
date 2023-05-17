use std::collections::HashMap;

use egui::Pos2;
use serde::{Deserialize, Serialize};

use crate::{annotations, bitcoin::Txid, graph::Graph};

#[derive(Serialize, Deserialize)]
pub struct Project {
    annotations: Annotations,
    transactions: Vec<Transaction>,
}

impl Project {
    pub fn export(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    pub fn import_(string: &str) -> Result<(annotations::Annotations, Vec<Transaction>), String> {
        let slf: Self = serde_json::from_str(string).map_err(|e| e.to_string())?;
        let annotations = annotations::Annotations::import_(&slf.annotations)?;
        Ok((annotations, slf.transactions))
    }

    pub fn new(graph: &Graph, annotations: &annotations::Annotations) -> Self {
        Self {
            annotations: annotations.export(),
            transactions: graph.export(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Annotations {
    pub tx_color: HashMap<String, [u8; 3]>,
    pub tx_label: HashMap<String, String>,
    pub coin_color: HashMap<String, [u8; 3]>,
    pub coin_label: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub txid: Txid,
    pub position: Position,
}

#[derive(Serialize, Deserialize)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn from_pos2(pos: Pos2) -> Self {
        Self {
            x: pos.x.round() as i32,
            y: pos.y.round() as i32,
        }
    }

    pub fn to_pos2(&self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }
}
