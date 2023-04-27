use crate::error::Result;
use crate::{
    error::Error,
    server::{Input, Output, Transaction},
};
use bitcoin::hashes::Hash;
use bitcoin_explorer::{Address, BitcoinDB, FConnectedTransaction, SBlock, STransaction, Txid};
use rocksdb;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct Txo {
    pub txid: Txid,
    pub vout: u32,
}

pub struct Store {
    db: rocksdb::DB,
    pub bitcoin: BitcoinDB,
}

impl Store {
    pub fn new<P: AsRef<Path>>(db_path: P, btc_path: P) -> Result<Self> {
        Ok(Self {
            db: rocksdb::DB::open_default(db_path)?,
            bitcoin: BitcoinDB::new(btc_path.as_ref(), true)?,
        })
    }

    pub fn set_txid_block_height(&self, txid: Txid, block_height: u32) -> Result<()> {
        self.db.put(txid, serde_cbor::to_vec(&block_height)?)?;
        Ok(())
    }

    pub fn get_txid_block_height(&self, txid: Txid) -> Result<Option<u32>> {
        if let Some(b) = self.db.get(txid)? {
            Ok(Some(serde_cbor::from_slice(&b)?))
        } else {
            Ok(None)
        }
    }

    pub fn set_spending_txid(&self, txid: Txid, vout: u32, spending_txid: Txid) -> Result<()> {
        let bytes = serde_cbor::to_vec(&Txo { txid, vout })?;
        self.db.put(bytes, spending_txid)?;
        Ok(())
    }

    pub fn get_spending_txid(&self, txid: Txid, vout: u32) -> Result<Option<Txid>> {
        let bytes = serde_cbor::to_vec(&Txo { txid, vout })?;
        if let Some(txbytes) = self.db.get(bytes)? {
            Ok(Some(Txid::from_hash(Hash::from_slice(&txbytes)?)))
        } else {
            Ok(None)
        }
    }

    pub fn get_tx(&self, txid: Txid) -> Result<Transaction> {
        let tx: STransaction = self.bitcoin.get_transaction(&txid)?;
        let connected_tx: FConnectedTransaction = self.bitcoin.get_connected_transaction(&txid)?;

        let block_height = self
            .get_txid_block_height(txid)?
            .ok_or(Error::NotYetIndexed)?;
        let block: SBlock = self.bitcoin.get_block(block_height as usize)?;

        let result = Transaction {
            timestamp: block.header.time,
            block_height,
            txid: txid.to_string(),
            inputs: connected_tx
                .input
                .iter()
                .enumerate()
                .map(|(i, input)| {
                    let address = Address::from_script(
                        &input.script_pubkey,
                        bitcoin_explorer::Network::Bitcoin,
                    );
                    Input {
                        txid: tx.input[i].txid,
                        vout: tx.input[i].vout,
                        value: input.value,
                        address: address
                            .clone()
                            .map_or("????".to_string(), |a| a.to_string()),
                        address_type: address.map_or("unknown".to_string(), |a| {
                            a.address_type().map_or("?".to_string(), |t| t.to_string())
                        }),
                    }
                })
                .collect(),
            outputs: connected_tx
                .output
                .iter()
                .enumerate()
                .map(|(o, output)| {
                    let address = Address::from_script(
                        &output.script_pubkey,
                        bitcoin_explorer::Network::Bitcoin,
                    );
                    Ok(Output {
                        spending_txid: self.get_spending_txid(txid, o as u32)?,
                        value: output.value,
                        address: address
                            .clone()
                            .map_or("????".to_string(), |a| a.to_string()),
                        address_type: address.map_or("unknown".to_string(), |a| {
                            a.address_type().map_or("?".to_string(), |t| t.to_string())
                        }),
                    })
                })
                .collect::<Result<Vec<Output>>>()?,
        };

        Ok(result)
    }

    pub fn commit_block_height(&self, height: u32) -> Result<()> {
        self.db.put("block_height", serde_cbor::to_vec(&height)?)?;
        Ok(())
    }

    pub fn get_committed_block_height(&self) -> Result<Option<u32>> {
        if let Some(bytes) = self.db.get("block_height")? {
            Ok(Some(serde_cbor::from_slice(&bytes)?))
        } else {
            Ok(None)
        }
    }
}
