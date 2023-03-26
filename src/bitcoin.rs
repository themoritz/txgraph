use std::net::TcpStream;

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
