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

    pub fn import_(string: String) -> Result<Self, String> {
        serde_json::from_str(&string).map_err(|e| e.to_string())
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
    pub tx_color: HashMap<Txid, [u8; 3]>,
    pub tx_label: HashMap<Txid, String>,
    pub coin_color: HashMap<(Txid, usize), [u8; 3]>,
    pub coin_label: HashMap<(Txid, usize), String>,
}

#[derive(Serialize, Deserialize)]
pub struct Transaction {
    pub txid: Txid,
    pub position: Pos2,
}
