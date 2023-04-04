use std::fmt::{Debug, Display};

use hex::{FromHex, ToHex};
use hyper::{body::to_bytes, client::HttpConnector, Body, Client};
use serde::{Deserialize, Serialize};
use tokio::runtime::Runtime;

#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct Txid([u8; 32]);

impl Txid {
    pub fn new(string: &str) -> Self {
        Self(<[u8; 32]>::from_hex(string).unwrap())
    }

    pub fn to_hex_string(&self) -> String {
        self.0.encode_hex()
    }
}

impl Display for Txid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex_string())
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
        Ok(Self::new(&string))
    }
}

impl Serialize for Txid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_hex_string())
    }
}

pub trait BitcoinData {
    fn get_transaction(&self, txid: Txid) -> Transaction;
}

pub struct HttpClient {
    client: hyper::Client<HttpConnector, Body>,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            client: Client::new(),
        }
    }
}

impl BitcoinData for HttpClient {
    fn get_transaction(&self, txid: Txid) -> Transaction {
        let uri = format!("http://127.0.0.1:1337/{}", txid).parse().unwrap();
        let rt = Runtime::new().unwrap();
        rt.block_on(async {
            let mut resp = self.client.get(uri).await.unwrap();
            serde_json::from_slice(&to_bytes(resp.body_mut()).await.unwrap()).unwrap()
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub timestamp: u32,
    pub txid: Txid,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Input {
    pub txid: Txid,
    pub vout: u32,
    pub value: u64,
    pub address: String,
    pub address_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Output {
    pub spending_txid: Option<Txid>,
    pub value: u64,
    pub address: String,
    pub address_type: String,
}

impl Transaction {
    pub fn amount(&self) -> u64 {
        self.inputs.iter().map(|input| input.value).sum()
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
        rem = rem % 1_000_000;
        let msats = if msats0 > 0 { Some(msats0) } else { None };
        let ksats0 = rem / 1_000;
        rem = rem % 1_000;
        let ksats = if ksats0 > 0 { Some(ksats0) } else { None };
        let sats = rem;

        let mut vec = Vec::new();
        let mut btc_to_go = btc;

        while btc_to_go > 0 {
            rem = btc_to_go % 1_000;
            btc_to_go = btc_to_go / 1_000;
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

        if btc.len() > 0 {
            write!(f, "{}", btc[0])?;
            started = true;

            for amount in btc.iter().skip(1) {
                if started {
                    write!(f, ",{:03}", amount)?;
                } else {
                    write!(f, ",{}", amount)?;
                }
            }

            write!(f, "'")?;
        }

        if started {
            write!(f, "{:02},", msats.unwrap_or(0))?;
        } else {
            if let Some(m) = msats {
                write!(f, "{},", m)?;
                started = true;
            }
        }

        if started {
            write!(f, "{:03},", ksats.unwrap_or(0))?;
        } else {
            if let Some(k) = ksats {
                write!(f, "{},", k)?;
                started = true
            }
        }

        if started {
            write!(f, "{:03}", sats)?;
        } else {
            write!(f, "{}", sats)?;
        }

        Ok(())
    }
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
                .to_hex_string(),
            "afe8d3199cd68f973a7cba01cb6b59f733864b782e9be49f61bb7f3d928a8382"
        );
    }
}
