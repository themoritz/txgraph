use std::{net::SocketAddr, path::Path, sync::Arc};

use bitcoin_explorer::FBlock;
use chrono::Utc;
use coin_index::{
    error::{Error, Result},
    server,
    store::Store,
};
use futures::TryFutureExt;
use hyper::{
    service::{make_service_fn, service_fn},
    Server,
};
use hyper_staticfile::Static;
use simple_logger::SimpleLogger;

type GenericError = Box<dyn std::error::Error + Send + Sync>;

struct Options {
    restart: bool,
    dev: bool,
    address: SocketAddr,
    bitcoin_dir: String,
    index_path: String,
    static_files: String,
}

fn parse_options() -> core::result::Result<Options, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    let options = Options {
        restart: pargs.contains(["-r", "--restart"]),
        dev: pargs.contains(["-d", "--dev"]),
        address: pargs
            .opt_value_from_fn("--address", |s| s.parse())?
            .unwrap_or("127.0.0.1:1337".parse().unwrap()),
        bitcoin_dir: pargs.value_from_str("--bitcoin-dir")?,
        index_path: pargs.value_from_str("--index-path")?,
        static_files: pargs.value_from_str("--static-files")?,
    };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        log::warn!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(options)
}

#[tokio::main]
async fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let options = parse_options().unwrap_or_else(|e| {
        log::error!("Error: {}.", e);
        std::process::exit(1);
    });

    let store = Arc::new(Store::new(&options.index_path, &options.bitcoin_dir)?);
    let store2 = store.clone();

    let scan = async {
        match tokio::task::spawn_blocking(move || scan_blockchain(store, options.restart)).await {
            Ok(Ok(result)) => Ok(result),
            Ok(Err(err)) => Err(err),
            Err(err) => Err(Error::from(err)),
        }
    };

    let static_ = Static::new(Path::new(&options.static_files));

    let service = make_service_fn(move |_| {
        let store = store2.clone();
        let static_ = static_.clone();
        async move {
            Ok::<_, GenericError>(service_fn(move |req| {
                server::server(static_.to_owned(), store.to_owned(), options.dev, req)
            }))
        }
    });

    let server = Server::bind(&options.address)
        .serve(service)
        .map_err(Error::from);
    log::info!("Listening on http://{}", options.address);

    tokio::try_join!(server, scan)?;

    Ok(())
}

fn scan_blockchain(store: Arc<Store>, restart: bool) -> Result<()> {
    let block_count = store.bitcoin.get_block_count();
    let start_block = if restart {
        0
    } else {
        store
            .get_committed_block_height()?
            .map_or(0, |h| h as usize)
    };

    let mut current_block = start_block;
    let mut n_txs = 0;
    let mut n_blocks = 0;
    let mut time = Utc::now();

    for block in store.bitcoin.iter_block::<FBlock>(start_block, block_count) {
        for tx in block.txdata {
            for i in tx.input {
                store.set_spending_txid(i.previous_output.txid, i.previous_output.vout, tx.txid)?;
            }

            store.set_txid_block_height(tx.txid, current_block as u32)?;

            n_txs += 1;
            if n_txs == 100_000 {
                let new_time = Utc::now();
                let time_diff = ((new_time - time).num_milliseconds() as f64) / 1_000.0;
                log::info!(
                    "Block: {:>6}, Txs/s: {:>6.0}, blocks/s: {:>6.0}, Example Tx: {}",
                    current_block,
                    n_txs as f64 / time_diff,
                    n_blocks as f64 / time_diff,
                    tx.txid
                );
                time = new_time;
                n_txs = 0;
                n_blocks = 0;
            }
        }

        current_block += 1;
        n_blocks += 1;

        if current_block % 100 == 0 {
            store.commit_block_height(current_block as u32)?;
        }
    }

    Ok(())
}
