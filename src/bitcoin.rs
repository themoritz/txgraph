use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use egui::{text::LayoutJob, Widget};
use hex::{FromHex, ToHex};
use serde::{Deserialize, Serialize};

use crate::{graph::sats_layout, platform::inner::get_random_int, style::Style};

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

    pub fn random_interesting() -> Self {
        let (_name, txid) = Self::INTERESTING_TXS[get_random_int(Self::INTERESTING_TXS.len())];
        Self::new(txid).unwrap()
    }

    pub const INTERESTING_TXS: [(&'static str, &'static str); 18] = [
        (
            "First Bitcoin",
            "0e3e2357e806b6cdb1f70b54c3a3a17b6714ee1f0e68bebb44a74b1efd512098",
        ),
        (
            "First TX (Satoshi to Hal Finney)",
            "f4184fc596403b9d638783cf57adfe4c75c605f6356fbc91338530e9831e9e16",
        ),
        (
            "10.000 BTC pizza",
            "a1075db55d416d3ca199f55b6084e2115b9345e16c5cf302fc80e9d5fbf5d48d",
        ),
        (
            "Whirlpool",
            "323df21f0b0756f98336437aa3d2fb87e02b59f1946b714a7b09df04d429dec2",
        ),
        (
            "Wasabi",
            "b3dcc5d68e7ba4946e8e7fec0207906fba89ccb4768112a25d6e6941f2e99d97",
        ),
        (
            "Wasabi post-mix spending",
            "4f89d6599fd1d728a78972d96930b8fca55e060aca9a04171b6c703c88285325",
        ),
        (
            "DarkWallet",
            "8e56317360a548e8ef28ec475878ef70d1371bee3526c017ac22ad61ae5740b8",
        ),
        (
            "MTGox 424242.42424242",
            "3a1b9e330d32fef1ee42f8e86420d2be978bbe0dc5862f17da9027cf9e11f8c4",
        ),
        (
            "Basic transaction",
            "2f17c08654e518f3ee46dd1438b58ef52b772e8cbc446b96b123d680a80bc3f7",
        ),
        (
            "Non-deterministic TX",
            "015d9cf0a12057d009395710611c65109f36b3eaefa3a694594bf243c097f404",
        ),
        (
            "Complex TX",
            "722d83ae4183ee17704704bdf31d9e77e6964387f657bbc0e09810a84a7fbad2",
        ),
        (
            "JoinMarket",
            "ca48b14f0a836b91d8719c51e50b313b425356a87111c4ed2cd6d81f0dbe60de",
        ),
        (
            "Weak CoinJoin",
            "a9b5563592099bf6ed68e7696eeac05c8cb514e21490643e0b7a9b72dac90b07",
        ),
        (
            "Address reuse",
            "0f7bf562c8768454077f9b5c6fe0c4c55c9a34786ad7380e00c2d8d00ebf779d",
        ),
        (
            "Block reward",
            "2157b554dcfda405233906e461ee593875ae4b1b97615872db6a25130ecc1dd6",
        ),
        (
            "Input/output merges",
            "03a858678475235b8b35a67495d67b65d5f2323236571aba3395f57eac57d72d",
        ),
        (
            "Multisig + address reuse",
            "dbbd98e638cc69a771fff79b34f5c6d59f08366f2238472c82d68b63757e051a",
        ),
        (
            "Taproot",
            "83c8e0289fecf93b5a284705396f5a652d9886cbd26236b0d647655ad8a37d82",
        ),
    ];
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
    #[allow(clippy::inconsistent_digit_grouping)]
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

pub struct SatsDisplay<'a> {
    sats: Sats,
    style: &'a Style,
}

impl<'a> SatsDisplay<'a> {
    pub fn new(sats: Sats, style: &'a Style) -> Self {
        Self { sats, style }
    }
}

impl Widget for SatsDisplay<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let mut job = LayoutJob::default();
        sats_layout(&mut job, &self.sats, self.style);
        ui.label(job)
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

            write!(f, ".")?;
        }

        if started {
            write!(f, "{:02} ", msats.unwrap_or(0))?;
        } else if let Some(m) = msats {
            write!(f, "{} ", m)?;
            started = true;
        }

        if started {
            write!(f, "{:03} ", ksats.unwrap_or(0))?;
        } else if let Some(k) = ksats {
            write!(f, "{} ", k)?;
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

#[allow(dead_code)]
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
    #[allow(clippy::inconsistent_digit_grouping)]
    fn sats() {
        let cases = vec![
            (42, "42"),
            (5_001, "5 001"),
            (19_010_020, "19 010 020"),
            (1_00_000_000, "1.00 000 000"),
            (4_001_01_123_456, "4,001.01 123 456"),
            (1_000_000_00_000_000, "1,000,000.00 000 000"),
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
