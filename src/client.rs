use egui::{Context, Id};
use serde::Deserialize;

use crate::{loading::Loading, notifications::Notifications};

#[derive(Clone)]
pub struct Client {
    base_url: String,
}

impl Client {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
        }
    }

    fn load(ctx: &Context) -> Self {
        ctx.data(|d| d.get_temp(Id::NULL))
            .unwrap_or(Self::new(env!("API_BASE")))
    }

    fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Id::NULL, self))
    }

    pub fn fetch_json<T: for<'de> Deserialize<'de>>(
        mk_request: impl FnOnce(&str) -> ehttp::Request,
        ctx: &Context,
        on_done: impl 'static + Send + FnOnce(),
        on_success: impl 'static + Send + FnOnce(T),
    ) {
        let slf = Self::load(ctx);

        Loading::start_loading(ctx);
        let request = mk_request(&slf.base_url);

        let ctx = ctx.clone();
        ehttp::fetch(request, move |response| {
            on_done();
            Loading::loading_done(&ctx);
            match response {
                Ok(response) => {
                    if response.status == 200 {
                        if let Some(text) = response.text() {
                            match serde_json::from_str(text) {
                                Ok(json) => on_success(json),
                                Err(err) => Notifications::error(
                                    &ctx,
                                    "Could not decode Api response.",
                                    Some(&err.to_string()),
                                ),
                            }
                        } else {
                            Notifications::error(&ctx, "Api response was empty.", None);
                        }
                    } else {
                        Notifications::error(
                            &ctx,
                            "Api request failed.",
                            Some(response.text().unwrap_or_default()),
                        );
                    }
                }
                Err(err) => {
                    Notifications::error(&ctx, "Api request failed.", Some(&err));
                }
            }
        });
    }
}
