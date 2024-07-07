use fluvio_connector_common::connector;

#[connector(config, name = "imap")]
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ImapConfig {
    pub host: String,
    pub port: String,
    pub user: String,
    pub password: String,
    pub mailbox: String,
    pub search: String,
    pub fetch: String,
    pub mode_bytes: bool,
    pub mode_utf8_lossy: bool,
    pub mode_parser: bool,
    pub mode_dkim_auth: bool,
    pub dkim_authenticated_move: Option<String>,
    pub dkim_unauthenticated_move: Option<String>,
    #[serde(default = "default_idle")]
    pub idle_timeout: u64,
    pub dangerous_cert: bool,
}

fn default_idle() -> u64 {
    300
}
