use std::sync::Arc;

use datalog::store::{Cardinality, Store, Type};
use egui::{Context, Id};
use egui::mutex::Mutex;

#[derive(Clone)]
struct Db(Arc<Mutex<Store>>);

impl Db {
    fn new() -> Self {
        let mut store = Store::new();
        store.add_attribute("name", Type::Str, Cardinality::One, "Name").unwrap();
        Self(Arc::new(Mutex::new(store)))
    }

    fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Id::NULL, self))
    }

    fn load(ctx: &Context) -> Self {
        ctx.data(|r| r.get_temp(Id::NULL)).unwrap_or_else(Self::new)
    }
}

pub trait DbExt {
    fn with_db<R, F: FnOnce(&mut Store) -> R>(&self, f: F) -> R;
}

impl DbExt for Context {
    fn with_db<R, F: FnOnce(&mut Store) -> R>(&self, f: F) -> R {
        let db = Db::load(self);
        let result = f(&mut db.0.lock());
        db.store(self);
        result
    }
}
