use std::{net::TcpStream, process::exit};

use electrum_client::{
    bitcoin::{hashes::hex::FromHex, Script, Txid},
    raw_client::RawClient,
    ElectrumApi, Error,
};
use rustls::{ClientConnection, StreamOwned};

pub fn get() -> Result<(), Error> {
    let bitcoin = Bitcoin::new("raspibolt.local:50002")?;
    let r = bitcoin.get_transaction(&Txid::from_hex(
        "89dea9f103d777e48c872bc9062373da7782e705386606281102d6278d01495f",
    )?)?;
    println!("{:#?}", r);
    exit(0);
}

#[derive(Debug)]
pub struct Input {
    txid: Txid,
    vout: u32,
    value: u64,
    script: Script,
}

#[derive(Debug)]
pub struct Output {
    value: u64,
    spend_txid: Option<Txid>,
    script: Script,
}

#[derive(Debug)]
pub struct Transaction {
    txid: Txid,
    inputs: Vec<Input>,
    outputs: Vec<Output>,
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

    pub fn get_transaction(self, txid: &Txid) -> Result<Transaction, Error> {
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
