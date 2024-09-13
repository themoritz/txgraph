#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result<()> {
    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).

    #[cfg(feature = "puffin")]
    let _puffin_server = {
        let server_addr = format!("127.0.0.1:{}", puffin_http::DEFAULT_PORT);
        let puffin_server = puffin_http::Server::new(&server_addr).unwrap();
        eprintln!("Run this to view profiling data:  puffin_viewer {server_addr}");
        puffin::set_scopes_on(true);
        puffin_server
    };

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "txgraph.info",
        native_options,
        Box::new(|cc| Ok(Box::new(txgraph::App::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let start_result = eframe::WebRunner::new()
            .start(
                "the_canvas_id",
                web_options,
                Box::new(|cc| Ok(Box::new(txgraph::App::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}
