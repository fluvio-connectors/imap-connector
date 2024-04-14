use crate::{config::ImapConfig, event::ImapEvent};
use anyhow::Result;
use async_std::channel::{self, Sender};
use async_std::task::spawn;
use async_trait::async_trait;
use async_std::net::TcpStream;
use async_native_tls::TlsConnector;
use async_imap::Client as AsyncImapClient;
use fluvio::Offset;
use fluvio_connector_common::tracing::{info, debug, trace};
use fluvio_connector_common::Source;
use futures::{stream::LocalBoxStream, StreamExt};
use std::str::FromStr;

const CHANNEL_BUFFER_SIZE: usize = 10000;

pub(crate) struct ImapSource {
    host: String,
    port: String,
    user: String,
    password: String,
    dangerous_cert: bool,
}

impl ImapSource {
    pub(crate) fn new(config: &ImapConfig) -> Result<Self> {
        let host = config.host.clone();
        let port = config.port.clone();
        let user = config.user.clone();
        let password = config.password.clone();
        let dangerous_cert = config.dangerous_cert;
        
        Ok(Self { host, port, user, password, dangerous_cert })
    }
}

#[async_trait]
impl<'a> Source<'a, String> for ImapSource {
    async fn connect(self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        info!("IMAP host: {} port {}", &self.host, &self.port);

        let (sender, receiver) = channel::bounded(CHANNEL_BUFFER_SIZE);
        spawn(imap_loop(sender, self.host, self.port, self.user, self.password, self.dangerous_cert));
        Ok(receiver.boxed_local())
    }
}

async fn imap_loop(tx: Sender<String>, host: String, port: String, user: String, password: String, dangerous_cert: bool) -> Result<()> {
    info!("Imap loop started");

    let idle_stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    let fetch_stream = TcpStream::connect(format!("{}:{}", host, port)).await?;

    info!("TCP TLS Connect");
    
    let mut idle_stream = TlsConnector::new()
        .use_sni(true)
        .danger_accept_invalid_certs(dangerous_cert)
        .connect(host.clone(), idle_stream)
        .await.unwrap();

    let mut fetch_stream = TlsConnector::new()
        .use_sni(true)
        .danger_accept_invalid_certs(dangerous_cert)
        .connect(host.clone(), fetch_stream)
            .await.unwrap();

    info!("Async IMAP Client Initialize");
    
    let idle_client = AsyncImapClient::new(idle_stream);
    let fetch_client = AsyncImapClient::new(fetch_stream);        
    
    let mut idle_session = idle_client
        .login(user.clone(), password.clone()).await
        .map_err(|(err, _client)| err)?;
    
    let mut fetch_session = fetch_client
        .login(user.clone(), password.clone()).await
        .map_err(|(err, _client)| err)?;


    let mut list = fetch_session.list(Some("*"), Some("*")).await.unwrap();

    while let Some(item) = list.next().await {

        info!("IMAP List item = {:?}", item.unwrap());
    }
    drop(list);
    
    let idle_inbox = idle_session.select("INBOX").await?;
    fetch_session.select("INBOX").await?;        
    
    info!("IMAP Connecting INBOX");

    debug!("IMAP idle_inbox = {:?}", idle_inbox);
    
    let mut idle_handle = idle_session.idle();

    loop {

        info!("IMAP Idling & Waiting for 5 minutes");
        
        idle_handle.init().await?;
        let (idle_fut, ss) = idle_handle.wait_with_timeout(std::time::Duration::from_secs(60 * 5));

        let idle_res = idle_fut.await?;

        info!("IMAP idle_res = {:?}", idle_res);
        

        
    }    
        /*
        while let Some(msg) = imap_sub.next().await {
            trace!("Imap got: {msg:?}");
            let imap_event: ImapEvent = msg.into();
            tx.send(imap_event.try_into()?).await?;
    }
        */

}
