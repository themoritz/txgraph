use std::{process::exit, net::TcpStream};

use electrum_client::{ElectrumApi, raw_client::RawClient, bitcoin::{Txid, hashes::hex::FromHex, Script}, Error};
use rustls::{StreamOwned, ClientConnection};

pub fn get() {
    let bitcoin = Bitcoin::new("raspibolt.local:50002").unwrap();
    let r = bitcoin.get_transaction(&Txid::from_hex("89dea9f103d777e48c872bc9062373da7782e705386606281102d6278d01495f").unwrap()).unwrap();
    println!("{:#?}", r);
    exit(0);
}

#[derive(Debug)]
pub struct Input {
    txid: Txid,
    vout: u32,
    value: u64
}

#[derive(Debug)]
pub struct Output {
    value: u64,
    spend_txid: Option<Txid>,
    script: Script
}

#[derive(Debug)]
pub struct Transaction {
    inputs: Vec<Input>,
    outputs: Vec<Output>
}

pub struct Bitcoin {
    client: RawClient<StreamOwned<ClientConnection, TcpStream>>
}

impl Bitcoin {
    pub fn new(addr: &str) -> Result<Self, Error> {
        Ok(Self { client: RawClient::new_ssl(addr, false, None)? })
    }

    pub fn get_transaction(self, txid: &Txid) -> Result<Transaction, Error> {
        let tx = self.client.transaction_get(txid)?;

        let input_txs = self.client.batch_transaction_get(tx.input.iter().map(|i| &i.previous_output.txid))?;
        let inputs: Vec<Input> = tx.input.iter().zip(input_txs).map(|(i, tx)| {
            Input {
                txid: tx.txid(),
                vout: i.previous_output.vout,
                value: tx.output[i.previous_output.vout as usize].value
            }
        }).collect();

        let output_histories = self.client.batch_script_get_history(tx.output.iter().map(|o| &o.script_pubkey))?;
        let outputs = tx.output.iter().enumerate().zip(output_histories).map(|((i, o), hist)| {
            let potential_txs = self.client.batch_transaction_get(hist.iter().map(|h| &h.tx_hash))?;
            Ok(Output {
                value: o.value,
                spend_txid: potential_txs.iter().find(|p| { p.input.iter().find(|inp| inp.previous_output.txid == *txid && inp.previous_output.vout as usize == i).is_some() }).map(|t| t.txid()),
                script: o.script_pubkey.clone()
            })
        }).collect::<Result<Vec<Output>, Error>>()?;

        Ok(Transaction { inputs, outputs })
    }
}
