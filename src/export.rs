use std::collections::HashMap;

use egui::Pos2;
use serde::{Deserialize, Serialize};

use crate::{annotations, bitcoin::Txid, graph::Graph, layout::Layout, transform::Transform};

// Public interface

#[derive(Default, PartialEq, Debug, Clone)]
pub struct Workspace {
    pub annotations: annotations::Annotations,
    pub layout: Layout0,
    pub transform: Transform0,
    pub transactions: Vec<Transaction>,
}

impl Workspace {
    pub fn new(
        graph: &Graph,
        annotations: &annotations::Annotations,
        layout: &Layout,
        transform: &Transform,
    ) -> Self {
        Self {
            annotations: (*annotations).clone(),
            layout: layout.export(),
            transform: transform.export(),
            transactions: graph.export(),
        }
    }
}

impl Serialize for Workspace {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        Workspace0 {
            version: 0,
            annotations: self.annotations.export(),
            layout: self.layout.clone(),
            transform: self.transform.clone(),
            transactions: self
                .transactions
                .iter()
                .map(Transaction::to_transaction0)
                .collect(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Workspace {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let workspace0 = Workspace0::deserialize(deserializer)?;
        Ok(Self {
            annotations: annotations::Annotations::import(&workspace0.annotations)
                .map_err(serde::de::Error::custom)?,
            layout: workspace0.layout,
            transform: workspace0.transform,
            transactions: workspace0
                .transactions
                .into_iter()
                .map(Transaction::from_transaction0)
                .collect(),
        })
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

// Version 0 of the workspace file format

fn validate_version<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<u32, D::Error> {
    let version = u32::deserialize(deserializer)?;
    if version == 0 {
        Ok(version)
    } else {
        Err(serde::de::Error::custom(format!(
            "Unsupported version: {}",
            version
        )))
    }
}

#[derive(Serialize, Deserialize)]
struct Workspace0 {
    #[serde(deserialize_with = "validate_version")]
    version: u32,
    annotations: Annotations0,
    #[serde(default)]
    layout: Layout0,
    #[serde(default)]
    transform: Transform0,
    transactions: Vec<Transaction0>,
}

// This is public because it's used in the conversion code in annotations.rs
#[derive(Serialize, Deserialize)]
pub struct Annotations0 {
    pub tx_color: HashMap<String, [u8; 3]>,
    pub tx_label: HashMap<String, String>,
    pub coin_color: HashMap<String, [u8; 3]>,
    pub coin_label: HashMap<String, String>,
}

// Public so that conversion code in layout.rs can use it.
#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct Layout0 {
    pub scale: u64,
    pub x1: u64,
    pub y1: u64,
    pub x2: u64,
    pub y2: u64,
}

impl Default for Layout0 {
    fn default() -> Self {
        Layout::default().export()
    }
}

// Public so that conversion code in transform.rs can use it.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Transform0 {
    pub z: f32,
    pub t_x: f32,
    pub t_y: f32,
}

impl PartialEq for Transform0 {
    fn eq(&self, other: &Self) -> bool {
        let eps = 0.00001;
        (self.z - other.z).abs() < eps
            && (self.t_x - other.t_x).abs() < eps
            && (self.t_y - other.t_y).abs() < eps
    }
}

impl Default for Transform0 {
    fn default() -> Self {
        Transform::default().export()
    }
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

    const WORKSPACE_FIXTURE_0: &str = r#"
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
            "layout": {
                "scale": 50,
                "x1": 1000000,
                "y1": 30,
                "x2": 10000000000000,
                "y2": 500
            },
            "transform": {
                "z": 1.3,
                "t_x": 30,
                "t_y": 2.99
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

    fn workspace_expected() -> Workspace {
        let txid =
            Txid::new("f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16").unwrap();
        let mut a = Annotations::default();
        a.set_tx_color(txid, Color32::from_rgb(0, 255, 0));
        a.set_tx_label(txid, "First Tx".to_string());
        a.set_coin_color((txid, 0), Color32::from_rgb(255, 0, 255));
        a.set_coin_label((txid, 0), "Output".to_string());

        Workspace {
            annotations: a,
            layout: Layout0 {
                scale: 50,
                x1: 1000000,
                y1: 30,
                x2: 10000000000000,
                y2: 500,
            },
            transform: Transform0 {
                z: 1.3,
                t_x: 30.0,
                t_y: 2.99,
            },
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
    fn test_workspace_fixture_0() {
        let actual = serde_json::from_str(&WORKSPACE_FIXTURE_0).unwrap();
        assert_eq!(workspace_expected(), actual);
    }

    #[test]
    fn test_workspace_roundtrip() {
        let expected = workspace_expected();
        let string = serde_json::to_string(&expected).unwrap();
        let actual = serde_json::from_str(&string).unwrap();
        assert_eq!(expected, actual);
    }
}
