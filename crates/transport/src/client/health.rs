use std::time::Duration;

use anyhow::Result;
use grammers_client::tl;
use grammers_mtsender::SenderPoolFatHandle;
use grammers_session::Session;

use super::ClientApi;
use super::watch::WatchdogTarget;

#[derive(Clone)]
pub struct HealthWatcher {
    api: ClientApi,
    handle: SenderPoolFatHandle,
}

impl HealthWatcher {
    pub fn new(api: ClientApi, handle: SenderPoolFatHandle) -> Self {
        Self { api, handle }
    }
}

impl WatchdogTarget for HealthWatcher {
    fn ping(&self, timeout: Duration) -> super::watch::BoxFuture<'_, bool> {
        Box::pin(async move {
            let probe = self.api.raw().invoke(&tl::functions::help::GetNearestDc {});
            matches!(tokio::time::timeout(timeout, probe).await, Ok(Ok(_)))
        })
    }

    fn reconnect(&self) -> super::watch::BoxFuture<'_, Result<()>> {
        Box::pin(async move {
            let dc_id = self.api.session.home_dc_id()?;
            self.handle.disconnect_from_dc(dc_id);
            self.api.prime_peer_cache().await;
            Ok(())
        })
    }

    fn give_up(&self) {
        std::process::exit(1);
    }
}
