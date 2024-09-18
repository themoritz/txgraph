use std::sync::Arc;

use egui::{
    ahash::{HashSet, HashSetExt},
    mutex::Mutex,
    Context, Id, Ui,
};

use crate::bitcoin::Txid;

#[derive(Clone)]
struct State {
    txids: Arc<Mutex<HashSet<Txid>>>,
    other: bool,
}

impl State {
    fn new() -> Self {
        Self {
            txids: Arc::new(Mutex::new(HashSet::new())),
            other: false,
        }
    }

    fn load(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp(Id::NULL)).unwrap_or_else(Self::new)
    }

    fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Id::NULL, self))
    }

    fn start_loading_txid(&self, txid: Txid) {
        self.txids.lock().insert(txid);
    }

    fn loading_txid_done(&self, txid: Txid) {
        self.txids.lock().remove(&txid);
    }

    fn is_loading(&self) -> bool {
        !self.txids.lock().is_empty() || self.other
    }

    fn is_txid_loading(&self, txid: &Txid) -> bool {
        self.txids.lock().contains(txid)
    }
}

pub struct Loading;

impl Loading {
    fn modify(ctx: &Context, f: impl FnOnce(&mut State)) {
        let mut state = State::load(ctx);
        f(&mut state);
        state.store(ctx);
    }

    pub fn start_loading_txid(ctx: &Context, txid: Txid) {
        Self::modify(ctx, |store| store.start_loading_txid(txid));
    }

    pub fn loading_txid_done(ctx: &Context, txid: Txid) {
        Self::modify(ctx, |store| store.loading_txid_done(txid));
    }

    pub fn start_loading(ctx: &Context) {
        Self::modify(ctx, |store| store.other = true);
    }

    pub fn loading_done(ctx: &Context) {
        Self::modify(ctx, |store| store.other = false);
    }

    pub fn spinner(ui: &mut Ui) {
        let state = State::load(ui.ctx());
        if state.is_loading() {
            ui.spinner();
        }
    }

    pub fn is_txid_loading(ui: &Ui, txid: &Txid) -> bool {
        State::load(ui.ctx()).is_txid_loading(txid)
    }
}
