#[allow(unused_imports)]
use fluvio_connector_common::tracing::{debug, error, info, trace, warn};

use crate::config::ImapConfig;
use crate::event::ImapEnvelope;
use crate::event::ImapEvent;
use async_imap::types::Fetch;
use msg_auth_status::alloc_yes::MessageAuthStatus;
use msg_auth_status::alloc_yes::{ReturnPathVerifier, ReturnPathVerifierStatus};

use anyhow::Result;
use std::io::{Error, ErrorKind};

// Fill the ImapEvent record with the FETCH record
pub(crate) fn fill_record<'msg>(
    config: &ImapConfig,
    uid: String,
    item: &'msg Fetch,
    do_dkim_auth: bool,
) -> Result<ImapEvent<'msg>> {
    let mut rec = ImapEvent::new(uid);
    if let Some(header) = item.header() {
        if config.mode_parser {
            let parsed = mail_parser::MessageParser::default().parse(header);
            rec.header_parsed = parsed;
        }
        if do_dkim_auth {
            let parsed = mail_parser::MessageParser::default().parse(header);
            if let Some(parsed) = parsed {
                let auth_status = match MessageAuthStatus::from_mail_parser(&parsed) {
                    Ok(auth_status) => auth_status,
                    Err(_) => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Could not create MessageAuthStatus from mail_parser",
                        )
                        .into())
                    }
                };
                let verifier = match ReturnPathVerifier::from_alloc_yes(&auth_status, &parsed) {
                    Ok(verifier) => verifier,
                    Err(_) => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "Return-Path header does not probably exist.",
                        )
                        .into())
                    }
                };
                match verifier.verify() {
                    Ok(ReturnPathVerifierStatus::Pass) => {
                        rec.dkim_authenticated = Some(true);
                    }
                    Ok(_) => {
                        rec.dkim_authenticated = Some(false);
                    }
                    Err(e) => {
                        rec.dkim_authenticated_error = Some(format!("{:?}", e));
                    }
                }
            }
        }
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
        if config.mode_parser {
            let parsed = mail_parser::MessageParser::default().parse(*body);
            rec.body_parsed = parsed;
        }
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
    // Move the mail in case Authenticated destination folder is set
    // and dkim_authenticated == true
    if let Some(dkim_move_to) = &config.dkim_authenticated_move {
        if let Some(true) = rec.dkim_authenticated {
            rec.moved_to = Some(dkim_move_to.clone());
        }
    }

    // Move the mail in case Unauthenticated destination folder is set
    // and dkim_authenticated == false
    if let Some(dkim_move_to) = &config.dkim_unauthenticated_move {
        if let Some(false) = rec.dkim_authenticated {
            rec.moved_to = Some(dkim_move_to.clone());
        }
    }
    Ok(rec)
}
