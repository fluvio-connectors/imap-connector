mod config;
mod event;
mod source;

use config::ImapConfig;

use fluvio::{RecordKey, TopicProducer};
use fluvio_connector_common::{
    connector,
    tracing::{debug, trace},
    Result, Source,
};
use futures::StreamExt;
use source::ImapSource;

#[connector(source)]
async fn start(config: ImapConfig, producer: TopicProducer) -> Result<()> {
    debug!(?config);
    let source = ImapSource::new(&config)?;
    let mut stream = source.connect(None).await?;
    while let Some(item) = stream.next().await {
        trace!(?item);
        producer.send(RecordKey::NULL, item).await?;
    }
    Ok(())
}
