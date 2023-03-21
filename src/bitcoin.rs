use std::net::TcpStream;

use egui::{Color32, Pos2, Rect, Rounding, Stroke};
use electrum_client::{
    bitcoin::{hashes::hex::FromHex, Script, Txid},
    raw_client::RawClient,
    ElectrumApi, Error,
};
use rustls::{ClientConnection, StreamOwned};

use crate::transform::Transform;

pub fn get() -> Result<(), Error> {
    let bitcoin = Bitcoin::new("raspibolt.local:50002")?;
    let r = bitcoin.get_transaction(&Txid::from_hex(
        "89dea9f103d777e48c872bc9062373da7782e705386606281102d6278d01495f",
    )?)?;
    println!("{:#?}", r);
    // exit(0);
    Ok(())
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

    fn scale(value: u64) -> f32 {
        f32::powf(value as f32, 1.0 / 2.0).round() / 10.0
    }

    pub fn draw(&self, painter: &egui::Painter, transform: &Transform) {
        let height: f32 = Self::scale(self.amount());
        let input_height: f32 = self
            .inputs
            .iter()
            .map(|input| Self::scale(input.value))
            .sum();
        let output_height = self
            .outputs
            .iter()
            .map(|output| Self::scale(output.value))
            .sum::<f32>()
            + Self::scale(self.fees());

        let start = 40.0_f32;

        let mut curr = start;

        for input in &self.inputs {
            let h = Self::scale(input.value) * height / input_height;
            painter.rect_stroke(
                transform.rect_to_screen(Rect::from_min_max(
                    Pos2::new(40.0, curr),
                    Pos2::new(50.0, curr + h),
                )),
                Rounding::none(),
                Stroke::new(1.0, Color32::BLACK),
            );
            curr += h;
        }

        curr = start;

        for output in &self.outputs {
            let h = Self::scale(output.value) * height / output_height;
            painter.rect_stroke(
                transform.rect_to_screen(Rect::from_min_max(
                    Pos2::new(60.0, curr),
                    Pos2::new(70.0, curr + h),
                )),
                Rounding::none(),
                Stroke::new(1.0, Color32::BLACK),
            );
            curr += h;
        }

        let h = Self::scale(self.fees()) * height / output_height;
        painter.rect_stroke(
            transform.rect_to_screen(Rect::from_min_max(
                Pos2::new(60.0, curr),
                Pos2::new(70.0, curr + h),
            )),
            Rounding::none(),
            Stroke::new(1.0, Color32::BLUE),
        );
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
