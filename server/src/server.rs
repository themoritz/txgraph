use std::{str::FromStr, sync::Arc};

use crate::store::Store;
use bitcoin_explorer::Txid;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use log::debug;
use serde::{Deserialize, Serialize};

pub async fn server(store: Arc<Store>, req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match req.method() {
        &Method::GET => {
            let path = req.uri().path();
            match Txid::from_str(&path[1..]) {
                Ok(txid) => {
                    debug!("{}", txid);
                    match store.get_tx(txid) {
                        Ok(tx) => {
                            let json = serde_json::to_string(&tx).unwrap();
                            let response = Response::builder()
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Body::from(json))
                                .unwrap();

                            Ok(response)
                        }
                        Err(err) => {
                            let response = Response::builder()
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::from(format!("Tx not found: {:?}", err)))
                                .unwrap();
                            return Ok(response);
                        }
                    }
                }
                Err(err) => {
                    let response = Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(Body::from(format!("Could not parse txid: {}", err)))
                        .unwrap();
                    return Ok(response);
                }
            }
        }
        _ => {
            let response = Response::builder()
                .status(StatusCode::METHOD_NOT_ALLOWED)
                .body(Body::empty())
                .unwrap();
            Ok(response)
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Transaction {
    pub timestamp: u32,
    pub block_height: u32,
    pub txid: String,
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
