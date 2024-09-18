use egui::{Context, Id};
use serde::Deserialize;

use crate::{loading::Loading, notifications::NotifyExt};

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
        on_done: impl 'static + Send + FnOnce(Result<T, FetchError>),
    ) {
        let slf = Self::load(ctx);

        Loading::start_loading(ctx);
        let request = mk_request(&slf.base_url);

        let ctx = ctx.clone();
        ehttp::fetch(request, move |response| {
            Loading::loading_done(&ctx);
            let result = match response {
                Ok(response) => {
                    if response.status == 200 {
                        if let Some(text) = response.text() {
                            match serde_json::from_str::<T>(text) {
                                Ok(json) => Ok(json),
                                Err(err) => Err(FetchError::DecodeFailed(err.to_string())),
                            }
                        } else {
                            Err(FetchError::ResponseEmpty)
                        }
                    } else {
                        Err(FetchError::RequestFailed(
                            response.text().unwrap_or_default().to_string(),
                        ))
                    }
                }
                Err(err) => Err(FetchError::RequestFailed(err)),
            };
            if let Err(ref err) = result {
                err.notify(&ctx);
            }
            on_done(result);
        });
    }
}

#[derive(Debug)]
pub enum FetchError {
    RequestFailed(String),
    DecodeFailed(String),
    ResponseEmpty,
}

impl FetchError {
    fn notify(&self, ctx: &Context) {
        match self {
            Self::RequestFailed(err) => {
                ctx.notify_error("Api request failed", Some(err));
            }
            Self::DecodeFailed(err) => {
                ctx.notify_error("Could not decode API response", Some(err));
            }
            Self::ResponseEmpty => {
                ctx.notify_error("API esponse was empty", None::<&str>);
            }
        }
    }
}
