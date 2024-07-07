use async_imap::imap_proto::Envelope as AsyncImapEnvelope;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct ImapEnvelope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Vec<AddressPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sender: Option<Vec<AddressPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to: Option<Vec<AddressPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<AddressPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cc: Option<Vec<AddressPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bcc: Option<Vec<AddressPart>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub in_reply_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AddressPart {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adl: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mailbox: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

impl From<&async_imap::imap_proto::types::Address<'_>> for AddressPart {
    fn from(address: &async_imap::imap_proto::types::Address<'_>) -> Self {
        let mut rec = AddressPart::default();
        if let Some(name) = &address.name {
            let str_name: String = String::from_utf8_lossy(name).into();
            rec.name = Some(str_name);
        }
        if let Some(adl) = &address.adl {
            let str_adl: String = String::from_utf8_lossy(adl).into();
            rec.adl = Some(str_adl);
        }
        if let Some(mailbox) = &address.mailbox {
            let str_mailbox: String = String::from_utf8_lossy(mailbox).into();
            rec.mailbox = Some(str_mailbox);
        }
        if let Some(host) = &address.host {
            let str_host: String = String::from_utf8_lossy(host).into();
            rec.host = Some(str_host);
        }
        rec
    }
}

impl From<&&AsyncImapEnvelope<'_>> for ImapEnvelope {
    fn from(envelope: &&AsyncImapEnvelope<'_>) -> Self {
        let mut rec = ImapEnvelope::default();
        if let Some(date) = &envelope.date {
            let str_date: String = String::from_utf8_lossy(date).into();
            rec.date = Some(str_date);
        }
        if let Some(subject) = &envelope.subject {
            let str_subject: String = String::from_utf8_lossy(subject).into();
            rec.subject = Some(str_subject);
        }
        if let Some(from) = &envelope.from {
            let vec_froms: Vec<AddressPart> = from.iter().map(|f| f.into()).collect();
            rec.from = Some(vec_froms);
        }
        if let Some(sender) = &envelope.sender {
            let vec_sender: Vec<AddressPart> = sender.iter().map(|f| f.into()).collect();
            rec.sender = Some(vec_sender);
        }
        if let Some(reply_to) = &envelope.reply_to {
            let vec_reply_to: Vec<AddressPart> = reply_to.iter().map(|f| f.into()).collect();
            rec.reply_to = Some(vec_reply_to);
        }
        if let Some(to) = &envelope.to {
            let vec_to: Vec<AddressPart> = to.iter().map(|f| f.into()).collect();
            rec.to = Some(vec_to);
        }
        if let Some(cc) = &envelope.cc {
            let vec_cc: Vec<AddressPart> = cc.iter().map(|f| f.into()).collect();
            rec.cc = Some(vec_cc);
        }
        if let Some(bcc) = &envelope.bcc {
            let vec_bcc: Vec<AddressPart> = bcc.iter().map(|f| f.into()).collect();
            rec.bcc = Some(vec_bcc);
        }
        if let Some(in_reply_to) = &envelope.in_reply_to {
            let str_in_reply_to: String = String::from_utf8_lossy(in_reply_to).into();
            rec.in_reply_to = Some(str_in_reply_to);
        }
        if let Some(message_id) = &envelope.message_id {
            let str_message_id: String = String::from_utf8_lossy(message_id).into();
            rec.message_id = Some(str_message_id);
        }
        rec
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ImapEvent<'msg> {
    pub uid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dkim_authenticated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dkim_authenticated_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moved_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internaldate: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_utf8_lossy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", borrow)]
    pub header_parsed: Option<mail_parser::Message<'msg>>,
    #[serde(skip_serializing_if = "Option::is_none", borrow)]
    pub body_parsed: Option<mail_parser::Message<'msg>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_utf8_lossy: Option<String>,
    // Placeholders
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub envelope: Option<ImapEnvelope>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body_structure: Option<bool>,
}

#[derive(Error, Debug)]
pub enum ImapEventError {
    #[error("Internal failure to convert Imap Event to JSON String: {0}")]
    InternalConversion(String),
}

impl<'msg> TryFrom<ImapEvent<'msg>> for String {
    type Error = ImapEventError;
    fn try_from(event: ImapEvent) -> Result<Self, ImapEventError> {
        serde_json::to_string(&event).map_err(|e| ImapEventError::InternalConversion(e.to_string()))
    }
}
