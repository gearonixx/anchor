// TODO: `anchor` CLI instead of `just`: anchor, anchor auth
use std::sync::Arc;

use anyhow::{Result, bail};
use chrono::Local;
use transport::client::watch::WatchdogConfig;
use transport::client::{AnchorClient, AnchorClientOptions};
use transport::config::Config;
use transport::event_loop;
use transport::gateway::{MessageGateImpl, handle_message};
use transport::google_callback;
use transport::logger;

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env()?;
    logger::init();
    log::info!("anchor.started at={}", Local::now().format("%Y-%m-%d %H:%M:%S"));

    let mut client = AnchorClient::connect(AnchorClientOptions {
        api_id: config.api_id,
        api_hash: config.api_hash.clone(),
        session_file: config.session_file.clone(),
    })
    .await?;

    if !client.is_authorized().await? { bail!("anchor not authorized."); }

    client.populate_caches().await;

    client.watch(WatchdogConfig::default());

    let gates = Arc::new(MessageGateImpl);
    let peers: Arc<[i64]> = config.peers.into();
    let google = Arc::new(config.google.clone());

    if let Some(app) = config.google.clone() {
        google_callback::spawn(client.api().clone(), app, config.google_callback_port, Arc::clone(&peers));
    }

    event_loop::spawn(
        client.api().clone(),
        Arc::clone(&peers),
        config.forced_probabilities,
        config.no_reply_limit,
        config.google.clone(),
    );

    client
        .run(move |api, message| {
            let gates = Arc::clone(&gates);
            let peers = Arc::clone(&peers);
            let google = Arc::clone(&google);

            async move {
                if let Err(err) = handle_message(&*gates, &api, &peers, google.as_ref().as_ref(), &message).await {
                    log::error!("anchor.gateway.failed: {err:#}");
                }
            }
        })
        .await?;

    log::info!("anchor.shutting_down");
    client.shutdown().await;
    Ok(())
}
