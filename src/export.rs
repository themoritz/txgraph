use std::collections::HashMap;

use egui::Pos2;
use serde::{Deserialize, Serialize};

use crate::{annotations, bitcoin::Txid, graph::Graph};

// Public interface

#[derive(Default, PartialEq, Eq, Debug, Clone)]
pub struct Project {
    pub annotations: annotations::Annotations,
    pub transactions: Vec<Transaction>,
}

impl Project {
    pub fn export(&self) -> String {
        Project0 {
            version: 0,
            annotations: self.annotations.export(),
            transactions: self
                .transactions
                .iter()
                .map(Transaction::to_transaction0)
                .collect(),
        }
        .export()
    }

    pub fn import(string: &str) -> Result<Self, String> {
        let project0 = Project0::import(string)?;
        Ok(Self {
            annotations: annotations::Annotations::import(&project0.annotations)?,
            transactions: project0
                .transactions
                .into_iter()
                .map(Transaction::from_transaction0)
                .collect(),
        })
    }

    pub fn new(graph: &Graph, annotations: &annotations::Annotations) -> Self {
        Self {
            annotations: (*annotations).clone(),
            transactions: graph.export(),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Transaction {
    pub txid: Txid,
    pub position: Pos2,
}

impl Transaction {
    pub fn new(txid: Txid, position: Pos2) -> Self {
        Self { txid, position }
    }

    fn from_transaction0(t: Transaction0) -> Self {
        Self {
            txid: t.txid,
            position: t.position.to_pos2(),
        }
    }

    fn to_transaction0(&self) -> Transaction0 {
        Transaction0 {
            txid: self.txid,
            position: Position0::from_pos2(self.position),
        }
    }
}

// Version 0 of the project file format

#[derive(Serialize, Deserialize)]
struct Project0 {
    #[serde(default)]
    version: u32,
    annotations: Annotations0,
    transactions: Vec<Transaction0>,
}

impl Project0 {
    fn export(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    fn import(string: &str) -> Result<Self, String> {
        let slf: Self = serde_json::from_str(string).map_err(|e| e.to_string())?;
        if slf.version == 0 {
            Ok(slf)
        } else {
            Err(format!("Unsupported version: {}", slf.version))
        }
    }
}

// This is public because it's used in the conversion code in annotations.rs
#[derive(Serialize, Deserialize)]
pub struct Annotations0 {
    pub tx_color: HashMap<String, [u8; 3]>,
    pub tx_label: HashMap<String, String>,
    pub coin_color: HashMap<String, [u8; 3]>,
    pub coin_label: HashMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct Transaction0 {
    txid: Txid,
    position: Position0,
}

#[derive(Serialize, Deserialize)]
struct Position0 {
    x: i32,
    y: i32,
}

impl Position0 {
    fn from_pos2(pos: Pos2) -> Self {
        Self {
            x: pos.x.round() as i32,
            y: pos.y.round() as i32,
        }
    }

    fn to_pos2(&self) -> Pos2 {
        Pos2::new(self.x as f32, self.y as f32)
    }
}

#[cfg(test)]
mod test {
    use self::annotations::Annotations;
    use egui::Color32;

    use super::*;

    const PROJECT_FIXTURE_0: &str = r#"
        {
            "version": 0,
            "annotations": {
                "tx_color": {
                    "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16": [0, 255, 0]
                },
                "tx_label": {
                    "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16": "First Tx"
                },
                "coin_color": {
                    "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16:0": [255, 0, 255]
                },
                "coin_label": {
                    "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16:0": "Output"
                }
            },
            "transactions": [
                {
                    "txid": "ea44e97271691990157559d0bdd9959e02790c34db6c006d779e82fa5aee708e",
                    "position": {
                        "x": 711,
                        "y": 351
                    }
                },
                {
                    "txid": "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16",
                    "position": {
                        "x": 755,
                        "y": 242
                    }
                }
            ]
        }
    "#;

    fn project_expected() -> Project {
        let txid =
            Txid::new("f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16").unwrap();
        let mut a = Annotations::default();
        a.set_tx_color(txid, Color32::from_rgb(0, 255, 0));
        a.set_tx_label(txid, "First Tx".to_string());
        a.set_coin_color((txid, 0), Color32::from_rgb(255, 0, 255));
        a.set_coin_label((txid, 0), "Output".to_string());

        Project {
            annotations: a,
            transactions: vec![
                Transaction {
                    txid: Txid::new(
                        "ea44e97271691990157559d0bdd9959e02790c34db6c006d779e82fa5aee708e",
                    )
                    .unwrap(),
                    position: Pos2::new(711.0, 351.0),
                },
                Transaction {
                    txid,
                    position: Pos2::new(755.0, 242.0),
                },
            ],
        }
    }

    #[test]
    fn test_project_fixture_0() {
        let actual = Project::import(PROJECT_FIXTURE_0).unwrap();
        assert_eq!(project_expected(), actual);
    }

    #[test]
    fn test_project_roundtrip() {
        let expected = project_expected();
        let string = expected.export();
        let actual = Project::import(&string).unwrap();
        assert_eq!(expected, actual);
    }
}
