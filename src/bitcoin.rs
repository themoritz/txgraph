use std::{fmt::Display, net::TcpStream};

use electrum_client::{
    bitcoin::{Script, Txid},
    raw_client::RawClient,
    ElectrumApi, Error,
};
use rustls::{ClientConnection, StreamOwned};

#[derive(Debug)]
pub struct Input {
    pub txid: Txid,
    pub vout: u32,
    pub value: u64,
    pub script: Script,
}

#[derive(Debug)]
pub struct Output {
    pub value: u64,
    pub spend_txid: Option<Txid>,
    pub script: Script,
}

#[derive(Debug)]
pub struct Transaction {
    pub txid: Txid,
    pub inputs: Vec<Input>,
    pub outputs: Vec<Output>,
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

pub struct Bitcoin {
    client: RawClient<StreamOwned<ClientConnection, TcpStream>>,
}

impl Bitcoin {
    pub fn new(addr: &str) -> Result<Self, Error> {
        Ok(Self {
            client: RawClient::new_ssl(addr, false, None)?,
        })
    }

    pub fn get_transaction(&self, txid: &Txid) -> Result<Transaction, Error> {
        let tx = self.client.transaction_get(txid)?;

        let inputs: Vec<Input> = {
            let previous_txs = self
                .client
                .batch_transaction_get(tx.input.iter().map(|i| &i.previous_output.txid))?;
            tx.input
                .iter()
                .zip(previous_txs)
                .map(|(input, tx)| {
                    let vout = input.previous_output.vout;
                    Input {
                        txid: tx.txid(),
                        vout,
                        value: tx.output[vout as usize].value,
                        script: tx.output[vout as usize].script_pubkey.clone(),
                    }
                })
                .collect()
        };

        let outputs = {
            let script_histories = self
                .client
                .batch_script_get_history(tx.output.iter().map(|o| &o.script_pubkey))?;
            tx.output
                .iter()
                .enumerate()
                .zip(script_histories)
                .map(|((i, output), history)| {
                    let history_txs = self
                        .client
                        .batch_transaction_get(history.iter().map(|h| &h.tx_hash))?;
                    let spend_txid = history_txs
                        .iter()
                        .find(|history_tx| {
                            history_tx
                                .input
                                .iter()
                                .find(|history_input| {
                                    history_input.previous_output.txid == *txid
                                        && history_input.previous_output.vout as usize == i
                                })
                                .is_some()
                        })
                        .map(|t| t.txid());
                    Ok(Output {
                        value: output.value,
                        spend_txid,
                        script: output.script_pubkey.clone(),
                    })
                })
                .collect::<Result<Vec<Output>, Error>>()?
        };

        Ok(Transaction {
            txid: *txid,
            inputs,
            outputs,
        })
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
    use crate::bitcoin::Sats;

    #[test]
    fn it_works() {
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
}
