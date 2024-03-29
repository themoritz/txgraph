use reunion::{UnionFind, UnionFindTrait};
use serde::{Deserialize, Serialize};

use crate::bitcoin::Txid;

#[derive(Serialize, Deserialize)]
pub struct Components {
    sets: UnionFind<Txid>,
}

impl Components {
    pub fn new() -> Self {
        Self {
            sets: UnionFind::new(),
        }
    }

    pub fn connected(&mut self, a: Txid, b: Txid) -> bool {
        self.sets.find(a) == self.sets.find(b)
    }

    pub fn connect(&mut self, a: Txid, b: Txid) {
        self.sets.union(a, b);
    }
}
