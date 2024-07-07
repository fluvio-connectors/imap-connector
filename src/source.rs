use crate::config::ImapConfig;
use anyhow::Result;
use async_imap::Client as AsyncImapClient;
use async_native_tls::TlsConnector;
use async_std::channel::{self, Sender};
use async_std::net::TcpStream;
use async_std::task::spawn;
use async_trait::async_trait;
use fluvio::Offset;
#[allow(unused_imports)]
use fluvio_connector_common::tracing::{debug, error, info, trace, warn};
use fluvio_connector_common::Source;
use futures::{stream::LocalBoxStream, StreamExt};

use std::time::SystemTime;

const CHANNEL_BUFFER_SIZE: usize = 10000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImapSource {
    config: ImapConfig,
}

impl ImapSource {
    pub(crate) fn new(config: ImapConfig) -> Result<Self> {
        Ok(Self { config })
    }
}

#[async_trait]
impl<'a> Source<'a, String> for ImapSource {
    async fn connect(self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        info!(
            "IMAP host: {} port {}",
            &self.config.host, &self.config.port
        );

        let (sender, receiver) = channel::bounded(CHANNEL_BUFFER_SIZE);
        spawn(imap_loop(sender, self.clone()));
        Ok(receiver.boxed_local())
    }
}

async fn imap_loop(tx: Sender<String>, source: ImapSource) -> Result<()> {
    debug!("Imap loop started");
    let config = source.config;

    let idle_stream = TcpStream::connect(format!("{}:{}", config.host, config.port)).await?;
    let fetch_stream = TcpStream::connect(format!("{}:{}", config.host, config.port)).await?;

    debug!("TCP TLS Connect");

    let idle_stream = TlsConnector::new()
        .use_sni(true)
        .danger_accept_invalid_certs(config.dangerous_cert)
        .connect(config.host.clone(), idle_stream)
        .await
        .unwrap();

    let fetch_stream = TlsConnector::new()
        .use_sni(true)
        .danger_accept_invalid_certs(config.dangerous_cert)
        .connect(config.host.clone(), fetch_stream)
        .await
        .unwrap();

    info!("Async IMAP Client Initialize");

    let idle_client = AsyncImapClient::new(idle_stream);
    let fetch_client = AsyncImapClient::new(fetch_stream);

    let mut idle_session = idle_client
        .login(config.user.clone(), config.password.clone())
        .await
        .map_err(|(err, _client)| err)?;

    let mut fetch_session = fetch_client
        .login(config.user.clone(), config.password.clone())
        .await
        .map_err(|(err, _client)| err)?;

    let do_dkim_auth = crate::imap_util::check_config(&config, &mut fetch_session).await?;

    let idle_inbox = idle_session.select(&config.mailbox).await?;

    info!("IMAP Connecting to Mailbox {}", &config.mailbox);
    debug!("IMAP idle_inbox cur = {:?}", idle_inbox);

    let mut idle_handle = idle_session.idle();
    idle_handle.init().await?;

    loop {
        let _fetch_inbox = fetch_session.select(&config.mailbox).await?;

        let search = fetch_session.uid_search(config.search.clone()).await?;

        let mut to_fetch = vec![];
        for search_item in &search {
            to_fetch.push(*search_item);
        }
        drop(search);

        let mut uid_moves: Vec<(String, String)> = vec![];

        debug!(
            "Checking {} found {} emails to fetch",
            &config.search,
            to_fetch.len()
        );

        for fetch_uid in &to_fetch {
            let uid = fetch_uid.to_string();

            debug!("Fetching UID {:?}", &uid);
            let mut fetch_new = fetch_session.uid_fetch(&uid, config.fetch.clone()).await?;

            while let Some(item_u) = fetch_new.next().await {
                let item = &item_u.unwrap();
                let rec = crate::record::fill_record(&config, uid.clone(), item, do_dkim_auth)?;

                // Move the mail between Mailboxes
                if let Some(ref move_to) = rec.moved_to {
                    info!("Moving {} to {}", &uid, &move_to);
                    uid_moves.push((uid.clone(), move_to.clone()));
                }

                tx.send(rec.try_into()?).await?;
            }
        }

        for (move_uid, move_to) in uid_moves.iter() {
            fetch_session.uid_mv(move_uid, move_to).await?;
        }

        let before = SystemTime::now();
        let mut cur_idle_msgs = 0;

        loop {
            cur_idle_msgs += 1;

            // Why is the server spamming so many non-interesting idle responses ?
            if cur_idle_msgs > 100 {
                error!("Idle response loop > 100 ?");
                break;
            }

            // We would like to awake ourselves despite idle messages flooding in
            let idle_left_secs = crate::imap_util::calculate_idle_left(before, config.idle_timeout);

            let (idle_fut, _ss) =
                idle_handle.wait_with_timeout(std::time::Duration::from_secs(idle_left_secs));

            let idle_res = idle_fut.await?;

            // If the idle response involves the maiblox, let's break and fetch new messages to check.
            if crate::imap_util::is_idle_response_interesting(&idle_res, &config.mailbox) {
                break;
            }
        }
    }
}
