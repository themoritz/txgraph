use std::{num::NonZeroUsize, sync::Arc};

use egui::{ahash::HashMap, mutex::Mutex, Context, Id};
use ehttp::Request;
use lru::LruCache;

use crate::{
    bitcoin::{Transaction, Txid},
    client::Client,
    loading::Loading,
};

const CACHE_SIZE: usize = 500;

#[derive(Clone)]
struct State {
    cache: Arc<Mutex<LruCache<Txid, Transaction>>>,
}

impl State {
    fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(LruCache::new(
                NonZeroUsize::new(CACHE_SIZE).unwrap(),
            ))),
        }
    }

    fn store(self, ctx: &Context) {
        let txs: Vec<Transaction> = self
            .cache
            .lock()
            .iter() // Most-recently used first, so need to rev
            .rev()
            .map(|(_, v)| v.clone())
            .collect();
        ctx.data_mut(|d| d.insert_persisted(Id::NULL, txs));
    }

    fn load(ctx: &Context) -> Self {
        let slf = Self::new();
        if let Some(txs) = ctx.data_mut(|d| d.get_persisted::<Vec<Transaction>>(Id::NULL)) {
            for tx in txs {
                slf.insert(tx.txid, tx);
            }
        }
        slf
    }

    fn get(&self, txid: &Txid) -> Option<Transaction> {
        self.cache.lock().get(txid).cloned()
    }

    fn insert(&self, txid: Txid, tx: Transaction) {
        self.cache.lock().put(txid, tx);
    }

    fn get_or_fetch(
        &self,
        ctx: &Context,
        txids: &[Txid],
        on_success: impl 'static + FnOnce(HashMap<Txid, Transaction>),
    ) {
        let (sender, receiver) = flume::unbounded();

        for &txid in txids {
            let slf = self.clone();
            let ctx2 = ctx.clone();
            let sender = sender.clone();
            if let Some(tx) = self.get(&txid) {
                sender.send(Ok(tx)).unwrap();
            } else {
                // Fetch tx from server
                Loading::start_loading_txid(ctx, txid);
                Client::fetch_json::<Transaction>(
                    move |base_url| {
                        let mut req = ehttp::Request::get(&format!("{}/tx/{}", base_url, txid));
                        authenticate(&mut req, &txid);
                        req
                    },
                    ctx,
                    move |result| {
                        Loading::loading_txid_done(&ctx2, txid);
                        if let Ok(ref tx) = result {
                            slf.insert(txid, tx.clone());
                        }
                        sender.send(result).unwrap();
                    },
                );
            }
        }

        let len_expected = txids.len();
        wasm_bindgen_futures::spawn_local(async move {
            let mut results = vec![];
            while let Ok(result) = receiver.recv_async().await {
                results.push(result);
                if results.len() == len_expected {
                    break;
                }
            }
            if let Ok(txs) = results.into_iter().collect::<Result<Vec<_>, _>>() {
                let map: HashMap<_, _> = txs.into_iter().map(|tx| (tx.txid, tx)).collect();
                on_success(map)
            }
        });
    }
}

pub struct TxCache;

impl TxCache {
    /// [on_success] is only called when all transactions have been fetched successfully.
    pub fn get_batch(
        ctx: &Context,
        txids: &[Txid],
        on_success: impl 'static + FnOnce(HashMap<Txid, Transaction>),
    ) {
        let state = State::load(ctx);
        let ctx2 = ctx.clone();
        let state2 = state.clone();
        state.get_or_fetch(ctx, txids, move |txs| {
            state2.store(&ctx2);
            on_success(txs);
        });
    }

    pub fn get(ctx: &Context, txid: Txid, on_success: impl 'static + FnOnce(Transaction)) {
        Self::get_batch(ctx, &vec![txid], move |txs| {
            if let Some(tx) = txs.get(&txid) {
                on_success(tx.clone());
            }
        });
    }
}

const API_TOKEN: &str = env!("API_TOKEN");

fn authenticate(request: &mut Request, txid: &Txid) {
    request
        .headers
        .insert("Authorization", format!("Bearer {API_TOKEN}"));
}
