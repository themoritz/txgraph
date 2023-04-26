use std::{str::FromStr, sync::Arc};

use crate::store::Store;
use bitcoin_explorer::Txid;
use hyper::{header, Body, Method, Request, Response, StatusCode};
use hyper_staticfile::Static;
use serde::{Deserialize, Serialize};

pub async fn server(
    static_: Static,
    store: Arc<Store>,
    dev: bool,
    req: Request<Body>,
) -> Result<Response<Body>, std::io::Error> {
    let builder = if dev {
        Response::builder().header(header::ACCESS_CONTROL_ALLOW_ORIGIN, "*")
    } else {
        Response::builder()
    };

    if req.uri().path().starts_with("/tx/") {
        match req.method() {
            &Method::GET => {
                let path = req.uri().path();
                match Txid::from_str(&path[4..]) {
                    Ok(txid) => match store.get_tx(txid) {
                        Ok(tx) => {
                            let json = serde_json::to_string(&tx).unwrap();
                            let response = builder
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(Body::from(json))
                                .unwrap();

                            Ok(response)
                        }
                        Err(err) => {
                            let response = builder
                                .status(StatusCode::NOT_FOUND)
                                .body(Body::from(format!("Tx not found: {:?}", err)))
                                .unwrap();
                            Ok(response)
                        }
                    },
                    Err(err) => {
                        let response = builder
                            .status(StatusCode::BAD_REQUEST)
                            .body(Body::from(format!("Could not parse txid: {}", err)))
                            .unwrap();
                        Ok(response)
                    }
                }
            }
            _ => {
                let response = builder
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .body(Body::empty())
                    .unwrap();
                Ok(response)
            }
        }
    } else {
        static_.serve(req).await
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
