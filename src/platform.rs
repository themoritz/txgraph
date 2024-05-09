#[cfg(target_arch = "wasm32")]
pub mod inner {
    use std::sync::mpsc::Sender;

    use egui::Vec2;
    use wasm_bindgen::{closure::Closure, prelude::wasm_bindgen};

    use crate::app::Update;
    use crate::bitcoin::Txid;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_name = addRouteListener)]
        fn add_route_listener_impl(callback: &Closure<dyn Fn(String)>);

        #[wasm_bindgen(js_name = pushHistoryState)]
        pub fn push_history_state(url: &str);

        #[wasm_bindgen(js_name = getRandom)]
        fn get_random() -> f64;
    }

    pub fn add_route_listener(sender: Sender<Update>, ctx: egui::Context) {
        let closure = Closure::new(move |url: String| {
            if let Some(txid) = url.strip_prefix("/tx/") {
                match Txid::new(txid) {
                    Ok(txid) => {
                        sender
                            .send(Update::LoadOrSelectTx { txid, pos: None })
                            .unwrap();
                        ctx.request_repaint();
                    }
                    Err(err) => {
                        sender
                            .send(Update::Error {
                                err: format!("{}: {}", url, err),
                            })
                            .unwrap();
                    }
                }
            } else if url == "/" {
            } else {
                sender
                    .send(Update::Error {
                        err: format!("Unknown route: {}", url),
                    })
                    .unwrap();
            }
        });

        add_route_listener_impl(&closure);
        closure.forget();
    }

    pub fn get_viewport_dimensions() -> Option<Vec2> {
        let window = web_sys::window()?;
        let width = window.inner_width().ok()?.as_f64()?;
        let height = window.inner_height().ok()?.as_f64()?;
        Some(Vec2::new(width as f32, height as f32))
    }

    pub fn get_random_vec2(range: f32) -> Vec2 {
        Vec2::new(
            get_random() as f32 * range - range / 2.0,
            get_random() as f32 * range - range / 2.0,
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub mod inner {
    use std::sync::mpsc::Sender;

    use egui::Vec2;
    use rand::{rngs::ThreadRng, Rng};

    use crate::app::Update;

    pub fn push_history_state(_url: &str) {}

    pub fn add_route_listener(_sender: Sender<Update>, _ctx: egui::Context) {}

    pub fn get_viewport_dimensions() -> Option<Vec2> {
        None
    }

    pub fn get_random_vec2(range: f32) -> Vec2 {
        let mut rng = ThreadRng::default();
        let half = range / 2.;
        Vec2::new(rng.gen_range(-half..half), rng.gen_range(-half..half))
    }
}