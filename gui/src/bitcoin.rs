use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct Txid([u8; 32]);

impl Txid {
    pub fn new(string: &str) -> Result<Self, String> {
        match <[u8; 32]>::from_hex(string) {
            Ok(bytes) => Ok(Self(bytes)),
            Err(e) => Err(e.to_string()),
        }
    }

    pub fn hex_string(&self) -> String {
        self.0.encode_hex()
    }

    pub fn chunks(&self) -> impl Iterator<Item = String> + '_ {
        (0..16).map(|i| {
            let x = &self.0[2 * i..2 * (i + 1)];
            x.encode_hex()
        })
    }
}

impl Display for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.hex_string())
    }
}

impl Debug for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'de> Deserialize<'de> for Txid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let string = String::deserialize(deserializer)?;
        Ok(Self::new(&string).unwrap()) // TODO better error handling
    }
}

impl Serialize for Txid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.hex_string())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub timestamp: i64,
    pub txid: Txid,
    pub block_height: u32,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    pub txid: Txid,
    pub vout: u32,
    pub value: u64,
    pub address: String,
    pub address_type: AddressType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AddressType {
    P2PKH,
    P2SH,
    P2WPKH,
    P2WSH,
    P2TR,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub spending_txid: Option<Txid>,
    pub value: u64,
    pub address: String,
    pub address_type: AddressType,
}

impl Transaction {
    pub fn is_coinbase(&self) -> bool {
        self.inputs.is_empty()
    }

    pub fn amount(&self) -> u64 {
        if self.is_coinbase() {
            // Special case for coinbase tx
            self.outputs.iter().map(|output| output.value).sum()
        } else {
            self.inputs.iter().map(|input| input.value).sum()
        }
    }

    pub fn fees(&self) -> u64 {
        let sent: u64 = self.outputs.iter().map(|output| output.value).sum();
        let fees = self.amount() as i64 - sent as i64;
        assert!(fees >= 0, "fees negative");
        fees as u64
    }
}

pub struct Sats(pub u64);

pub struct AmountComponents {
    pub sats: u64,
    pub ksats: Option<u64>,
    pub msats: Option<u64>,
    /// In write order.
    pub btc: Vec<u64>,
}

impl Sats {
    pub fn components(&self) -> AmountComponents {
        let btc = self.0 / 1_00_000_000;
        let mut rem = self.0 % 1_00_000_000;
        let msats0 = rem / 1_000_000;
        rem %= 1_000_000;
        let msats = if msats0 > 0 { Some(msats0) } else { None };
        let ksats0 = rem / 1_000;
        rem %= 1_000;
        let ksats = if ksats0 > 0 { Some(ksats0) } else { None };
        let sats = rem;

        let mut vec = Vec::new();
        let mut btc_to_go = btc;

        while btc_to_go > 0 {
            rem = btc_to_go % 1_000;
            btc_to_go /= 1_000;
            vec.push(rem);
        }

        vec.reverse();

        AmountComponents {
            sats,
            ksats,
            msats,
            btc: vec,
        }
    }
}

impl Display for Sats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let AmountComponents {
            sats,
            ksats,
            msats,
            btc,
        } = self.components();

        let mut started = false;

        if !btc.is_empty() {
            write!(f, "{}", btc[0])?;
            started = true;

            for amount in btc.iter().skip(1) {
                write!(f, ",{:03}", amount)?;
            }

            write!(f, "'")?;
        }

        if started {
            write!(f, "{:02},", msats.unwrap_or(0))?;
        } else if let Some(m) = msats {
            write!(f, "{},", m)?;
            started = true;
        }

        if started {
            write!(f, "{:03},", ksats.unwrap_or(0))?;
        } else if let Some(k) = ksats {
            write!(f, "{},", k)?;
            started = true
        }

        if started {
            write!(f, "{:03}", sats)?;
        } else {
            write!(f, "{}", sats)?;
        }

        Ok(())
    }
}

pub fn dummy_transactions() -> HashMap<Txid, Transaction> {
    let z = Txid::new("97ddfbbae6be97fd6cdf3e7ca13232a3affa2353e29badfab7f73011edd4ced9").unwrap();
    let a = Txid::new("97ddfbbae6be97fd6cdf3e7ca13232a3afff2353e29badfab7f73011edd4ced9").unwrap();
    let b = Txid::new("4a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b").unwrap();
    let c = Txid::new("1a5e1e4baab89f3a32518a88c31bc87f618f76673e2cc77ab2127b7afdeda33b").unwrap();
    HashMap::from([
        (
            a,
            Transaction {
                timestamp: 1,
                block_height: 0,
                txid: a,
                inputs: vec![Input {
                    value: 140_600_000,
                    txid: z,
                    vout: 0,
                    address: "fslkfjeslk".to_string(),
                    address_type: AddressType::P2PKH,
                }],
                outputs: vec![
                    Output {
                        spending_txid: Some(b),
                        value: 100_230_000,
                        address: "fsklefj".to_string(),
                        address_type: AddressType::P2PKH,
                    },
                    Output {
                        spending_txid: Some(c),
                        value: 12_300_000,
                        address: "fsklefj".to_string(),
                        address_type: AddressType::P2PKH,
                    },
                ],
            },
        ),
        (
            b,
            Transaction {
                timestamp: 2,
                block_height: 0,
                txid: b,
                inputs: vec![Input {
                    value: 100_230_000,
                    txid: a,
                    vout: 0,
                    address: "fslkfjeslk".to_string(),
                    address_type: AddressType::P2PKH,
                }],
                outputs: vec![Output {
                    spending_txid: Some(c),
                    value: 12_300_000,
                    address: "fsklefj".to_string(),
                    address_type: AddressType::P2PKH,
                }],
            },
        ),
        (
            c,
            Transaction {
                timestamp: 2,
                block_height: 0,
                txid: c,
                inputs: vec![
                    Input {
                        value: 12_300_000,
                        txid: a,
                        vout: 1,
                        address: "fslkfjeslk".to_string(),
                        address_type: AddressType::P2PKH,
                    },
                    Input {
                        value: 12_300_000,
                        txid: b,
                        vout: 0,
                        address: "fslkfjeslk".to_string(),
                        address_type: AddressType::P2PKH,
                    },
                ],
                outputs: vec![],
            },
        ),
    ])
}

#[cfg(test)]
mod tests {
    use crate::bitcoin::{Sats, Txid};

    #[test]
    fn sats() {
        let cases = vec![
            (42, "42"),
            (5_001, "5,001"),
            (19_010_020, "19,010,020"),
            (1_00_000_000, "1'00,000,000"),
            (4_001_01_123_456, "4,001'01,123,456"),
            (1_000_000_00_000_000, "1,000,000'00,000,000"),
        ];

        for case in cases {
            assert_eq!(format!("{}", Sats(case.0)), case.1);
        }
    }

    #[test]
    fn txid() {
        assert_eq!(
            Txid::new("afe8d3199cd68f973a7cba01cb6b59f733864b782e9be49f61bb7f3d928a8382")
                .unwrap()
                .hex_string(),
            "afe8d3199cd68f973a7cba01cb6b59f733864b782e9be49f61bb7f3d928a8382"
        );
    }
}
