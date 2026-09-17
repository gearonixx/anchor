use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use grammers_client::client::UpdatesConfiguration;
use grammers_client::session::storages::SqliteSession;
use grammers_client::{Client, SenderPool, SignInError, tl};
use grammers_mtsender::SenderPoolFatHandle;
use grammers_session::updates::UpdatesLike;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::{JoinHandle, JoinSet};

use super::{ClientApi, IncomingMessage};
use super::health::HealthWatcher;
use super::parser::MessageParser;
use super::watch::{Watchdog, WatchdogConfig};

#[derive(Debug, Clone, Copy)]
pub enum LoginPrompt {
    Code,
    Password,
}

#[derive(Clone)]
pub struct AnchorClientOptions {
    pub api_id: i32,
    pub api_hash: String,
    pub session_file: PathBuf,
}

type UpdateReceiver = UnboundedReceiver<UpdatesLike>;

pub struct AnchorClient {
    api: ClientApi,
    handle: SenderPoolFatHandle,
    pool_task: JoinHandle<()>,
    updates: Option<UpdateReceiver>,
    api_hash: String,
}

impl AnchorClient {
    pub async fn connect(opts: AnchorClientOptions) -> Result<Self> {
        // create the .anchor/ directory if missing so SQLite has somewhere to write the file.
        if let Some(dir) = opts.session_file.parent()
            && !dir.as_os_str().is_empty()
        {
            std::fs::create_dir_all(dir)
                .with_context(|| format!("creating session dir {}", dir.display()))?;
        }

        // SqliteSession::open creates the file
        let session = Arc::new(
            // the file exists now, but there is no session in the "logged in" sense yet
            SqliteSession::open(&opts.session_file)
                .await
                .with_context(|| format!("opening session {}", opts.session_file.display()))?,
        );

        let SenderPool {
            runner,
            handle,
            updates,
        } = SenderPool::new(Arc::clone(&session), opts.api_id);

        let client = Client::new(handle.clone());
        let pool_task = tokio::spawn(runner.run());

        Ok(Self {
            api: ClientApi::new(client, session),
            handle,
            pool_task,
            // updates is a Receiver that yields items one by one
            // updates = the incoming Telegram event queue.
            updates: Some(updates),
            api_hash: opts.api_hash,
        })
    }

    pub fn api(&self) -> &ClientApi {
        &self.api
    }

    pub async fn sign_in_with_prompt(
        &self,
        phone: &str,
        mut prompt: impl FnMut(LoginPrompt) -> Result<String>,
    ) -> Result<()> {
        let token = self.api.raw().request_login_code(phone, &self.api_hash).await?;
        let code = prompt(LoginPrompt::Code)?;

        match self.api.raw().sign_in(&token, &code).await {
            Ok(_) => Ok(()),
            Err(SignInError::PasswordRequired(token)) => {
                let password = prompt(LoginPrompt::Password)?;
                self.api.raw().check_password(token, password.trim()).await?;
                Ok(())
            }
            Err(err) => anyhow::bail!("sign-in failed: {err}"),
        }
    }

    pub async fn is_authorized(&self) -> Result<bool> {
        Ok(self.api.raw().is_authorized().await?)
    }

    pub async fn populate_caches(&self) {
        self.api.prime_peer_cache().await;
        log::info!("anchor.connected");
    }

    pub async fn wait_for_login_token(&mut self, timeout: Duration) -> bool {
        let Some(rx) = self.updates.as_mut() else {
            return false;
        };
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            match tokio::time::timeout_at(deadline, rx.recv()).await {
                Err(_) | Ok(None) => return false,
                Ok(Some(UpdatesLike::Updates(updates))) => {
                    if Self::carries_login_token(&updates) {
                        return true;
                    }
                }
                Ok(Some(_)) => {}
            }
        }
    }

    fn carries_login_token(updates: &tl::enums::Updates) -> bool {
        let is_token = |u: &tl::enums::Update| matches!(u, tl::enums::Update::LoginToken);
        match updates {
            tl::enums::Updates::UpdateShort(short) => is_token(&short.update),
            tl::enums::Updates::Updates(u) => u.updates.iter().any(is_token),
            tl::enums::Updates::Combined(u) => u.updates.iter().any(is_token),
            _ => false,
        }
    }

    pub fn watchdog_target(&self) -> HealthWatcher {
        HealthWatcher::new(self.api.clone(), self.handle.clone())
    }

    pub fn watch(&self, config: WatchdogConfig) -> JoinHandle<()> {
        tokio::spawn(Watchdog::new(self.watchdog_target(), config).run())
    }

    pub async fn run<F, Fut>(&mut self, handler: F) -> Result<()>
    where
        F: Fn(ClientApi, IncomingMessage) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let updates = self
            .updates
            .take()
            .context("update stream already consumed")?;

        let config = UpdatesConfiguration {
            // do not process old updates accumulated during downtime.
            // (debatable)
            catch_up: false,
            ..Default::default()
        };

        let mut stream = self
            .api
            .raw()
            .stream_updates(updates, config)
            .await
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let handler = Arc::new(handler);
        let mut tasks = JoinSet::new();

        log::info!("tg.updates.listening");

        // never block the loop; shut down gracefully at the end.
        // IN THE LOOP:
        // "reap whatever already finished and keep going."
        //
        // AT THE END:
        // "we are closing; wait for everything already started."

        loop {
            while tasks.try_join_next().is_some() {}

            tokio::select! {
                _ = tokio::signal::ctrl_c() => {
                    log::info!("tg.shutdown.signal");
                    break;
                }
                update = stream.next() => {
                    let update = match update {
                        Ok(u) => u,
                        Err(err) => {
                            log::error!("tg.updates.error: {err}");
                            continue;
                        }
                    };
                    if let Some(msg) = MessageParser::new(&update).parse() {
                        let api = self.api.clone();
                        let handler = Arc::clone(&handler);
                        tasks.spawn(async move { handler(api, msg).await });
                    }
                }
            }
        }

        if let Err(err) = stream.sync_update_state().await {
            log::warn!("tg.session.sync_failed: {err}");
        }

        while tasks.join_next().await.is_some() {}
        Ok(())
    }

    pub async fn shutdown(self) {
        self.handle.quit();
        let _ = self.pool_task.await;
    }
}
