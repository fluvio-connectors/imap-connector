use crate::{config::ImapConfig, event::ImapEnvelope, event::ImapEvent};
use anyhow::Result;
use async_imap::Client as AsyncImapClient;
use async_native_tls::TlsConnector;
use async_std::channel::{self, Sender};
use async_std::net::TcpStream;
use async_std::task::spawn;
use async_trait::async_trait;
use fluvio::Offset;
#[allow(unused_imports)]
use fluvio_connector_common::tracing::{debug, info, trace};
use fluvio_connector_common::Source;
use futures::{stream::LocalBoxStream, StreamExt};

const CHANNEL_BUFFER_SIZE: usize = 10000;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImapSource {
    host: String,
    port: String,
    user: String,
    password: String,
    mailbox: String,
    search: String,
    fetch: String,
    mode_bytes: bool,
    mode_utf8_lossy: bool,
    dangerous_cert: bool,
}

impl ImapSource {
    pub(crate) fn new(config: &ImapConfig) -> Result<Self> {
        let host = config.host.clone();
        let port = config.port.clone();
        let user = config.user.clone();
        let password = config.password.clone();
        let mailbox = config.mailbox.clone();
        let search = config.search.clone();
        let fetch = config.fetch.clone();
        let mode_utf8_lossy = config.mode_utf8_lossy;
        let mode_bytes = config.mode_bytes;
        let dangerous_cert = config.dangerous_cert;

        Ok(Self {
            host,
            port,
            user,
            password,
            mailbox,
            search,
            fetch,
            mode_utf8_lossy,
            mode_bytes,
            dangerous_cert,
        })
    }
}

#[async_trait]
impl<'a> Source<'a, String> for ImapSource {
    async fn connect(self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        info!("IMAP host: {} port {}", &self.host, &self.port);

        let (sender, receiver) = channel::bounded(CHANNEL_BUFFER_SIZE);
        spawn(imap_loop(sender, self.clone()));
        Ok(receiver.boxed_local())
    }
}

//async fn imap_loop(tx: Sender<String>, host: String, port: String, user: String, password: String, mailbox: String, dangerous_cert: bool) -> Result<()> {
async fn imap_loop(tx: Sender<String>, config: ImapSource) -> Result<()> {
    info!("Imap loop started");

    let idle_stream = TcpStream::connect(format!("{}:{}", config.host, config.port)).await?;
    let fetch_stream = TcpStream::connect(format!("{}:{}", config.host, config.port)).await?;

    info!("TCP TLS Connect");

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

    /* TODO
    let mut list = fetch_session.list(Some("*"), Some("*")).await.unwrap();
    while let Some(item) = list.next().await {

        info!("IMAP List item = {:?}", item.unwrap());
    }
    drop(list);
     */

    let idle_inbox = idle_session.select(&config.mailbox).await?;
    let _fetch_inbox = fetch_session.select(&config.mailbox).await?;

    info!("IMAP Connecting Mailbox {}", &config.mailbox);

    info!("IMAP idle_inbox = {:?}", idle_inbox);

    let mut idle_handle = idle_session.idle();

    loop {
        let search = fetch_session.uid_search(config.search.clone()).await?;

        let mut to_fetch = vec![];
        for search_item in &search {
            to_fetch.push(*search_item);
        }
        drop(search);

        for fetch_uid in &to_fetch {
            info!("Fetching UID {:?}", fetch_uid);
            let mut fetch_new = fetch_session
                .uid_fetch(fetch_uid.to_string(), config.fetch.clone())
                .await?;
            while let Some(item_u) = fetch_new.next().await {
                let mut rec = ImapEvent {
                    uid: *fetch_uid,
                    flags: None,
                    body: None,
                    body_utf8_lossy: None,
                    header: None,
                    header_utf8_lossy: None,
                    text: None,
                    envelope: None,
                    internaldate: None,
                    body_structure: None,
                };

                let item = &item_u.unwrap();

                if let Some(header) = item.header() {
                    if config.mode_bytes {
                        rec.header = Some(header.into());
                    }
                    if config.mode_utf8_lossy {
                        let string_header = String::from_utf8_lossy(header).to_string();
                        rec.header_utf8_lossy = Some(string_header);
                    }
                }
                if let Some(envelope) = &item.envelope() {
                    let imap_envelope: ImapEnvelope = envelope.into();
                    rec.envelope = Some(imap_envelope);
                }
                if let Some(body) = &item.body() {
                    if config.mode_bytes {
                        rec.body = Some(body.to_vec());
                    }
                    if config.mode_utf8_lossy {
                        let body_utf8_lossy: String = String::from_utf8_lossy(body).to_string();
                        rec.body_utf8_lossy = Some(body_utf8_lossy);
                    }
                }
                if let Some(internal_date) = &item.internal_date() {
                    rec.internaldate = Some(internal_date.to_rfc3339());
                }

                tx.send(rec.try_into()?).await?;
            }
        }

        info!("IMAP Idling & Waiting for 5 minutes");

        idle_handle.init().await?;
        let (idle_fut, _ss) = idle_handle.wait_with_timeout(std::time::Duration::from_secs(60 * 5));

        let idle_res = idle_fut.await?;

        info!("IMAP idle_res = {:?}", idle_res);
    }
}
