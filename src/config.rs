use fluvio_connector_common::connector;

#[connector(config, name = "imap")]
#[derive(Debug)]
pub(crate) struct ImapConfig {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
}
