use crate::{config::ImapConfig, event::ImapEvent};
use anyhow::Result;
use async_std::channel::{self, Sender};
use async_std::task::spawn;
use async_trait::async_trait;
use async_std::net::TcpStream;
use async_native_tls::TlsConnector;
use async_imap::Client as AsyncImapClient;
use fluvio::Offset;
use fluvio_connector_common::tracing::{info, trace};
use fluvio_connector_common::Source;
use futures::{stream::LocalBoxStream, StreamExt};

const CHANNEL_BUFFER_SIZE: usize = 10000;

pub(crate) struct ImapSource {
    host: String,
    port: String,
    user: String,
    password: String,
}

impl ImapSource {
    pub(crate) fn new(config: &ImapConfig) -> Result<Self> {
        let host = config.host.clone();
        let port = config.port.clone();
        let user = config.user.clone();
        let password = config.password.clone();
        
        Ok(Self { host, port, user, password })
    }
}

#[async_trait]
impl<'a> Source<'a, String> for ImapSource {
    async fn connect(self, _offset: Option<Offset>) -> Result<LocalBoxStream<'a, String>> {
        info!("IMAP host: {} port {}", &self.host, &self.port);

        let (sender, receiver) = channel::bounded(CHANNEL_BUFFER_SIZE);
        spawn(imap_loop(sender, self.host, self.port, self.user, self.password));
        Ok(receiver.boxed_local())
    }
}

async fn imap_loop(tx: Sender<String>, host: String, port: String, user: String, password: String) -> Result<()> {
    info!("Imap loop started");

    let idle_stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    let fetch_stream = TcpStream::connect(format!("{}:{}", host, port)).await?;
    
    let mut idle_stream = TlsConnector::new()
        .use_sni(true)
        .connect(host.clone(), idle_stream)
        .await?;

    let mut fetch_stream = TlsConnector::new()
        .use_sni(true)
        .connect(host.clone(), fetch_stream)
            .await?;
    
    let idle_client = AsyncImapClient::new(idle_stream);
    let fetch_client = AsyncImapClient::new(fetch_stream);        
    
    let mut idle_session = idle_client
        .login(user.clone(), password.clone()).await
        .map_err(|(err, _client)| err)?;
    
    let mut fetch_session = fetch_client
        .login(user.clone(), password.clone()).await
        .map_err(|(err, _client)| err)?;
    
    idle_session.select("INBOX").await?;
    fetch_session.select("INBOX").await?;        
    
    info!("IMAP Connecting INBOX");
    
    let mut idle_handle = idle_session.idle();
    
    loop {

        idle_handle.init().await?;
        let (idle_res, ss) = idle_handle.wait_with_timeout(std::time::Duration::from_secs(60 * 5));

        idle_res.await?;

        
        
    }    
        /*
        while let Some(msg) = imap_sub.next().await {
            trace!("Imap got: {msg:?}");
            let imap_event: ImapEvent = msg.into();
            tx.send(imap_event.try_into()?).await?;
    }
        */

}
