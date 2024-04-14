use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ImapEvent {
    pub mailbox: String,
    pub mail_data: Vec<u8>,
}

#[derive(Error, Debug)]
pub enum ImapEventError {
    #[error("Internal failure to convert Imap Event to JSON String: {0}")]
    InternalConversion(String),
}

impl TryFrom<ImapEvent> for String {
    type Error = ImapEventError;
    fn try_from(event: ImapEvent) -> Result<Self, ImapEventError> {
        serde_json::to_string(&event).map_err(|e| ImapEventError::InternalConversion(e.to_string()))
    }
}

/*
impl From<NNNN> for ImapEvent {
    fn from(n: NNN) -> Self {
        Self {
        }
    }
}
*/
