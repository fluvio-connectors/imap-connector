use crate::config::ImapConfig;
use anyhow::Result;
use async_imap::extensions::idle::IdleResponse;
use async_imap::imap_proto::types::{MailboxDatum, Response as ImapResponse};
use async_imap::Session as ImapSession;

#[allow(unused_imports)]
use fluvio_connector_common::tracing::{debug, info, trace, warn};

use async_std::io::{Read, Write};
use core::fmt;
use std::collections::HashMap;
use std::time::SystemTime;

use async_std::stream::StreamExt;

#[derive(Debug, Default)]
struct MboxCheck {
    exists: bool,
}

pub(crate) async fn check_config<T>(
    config: &ImapConfig,
    fetch_session: &mut ImapSession<T>,
) -> Result<bool>
where
    T: Read + Write + Unpin + Send + fmt::Debug,
{
    // Figure out if the message passed DKIM auth for Return-Path
    let mut do_dkim_auth = config.mode_dkim_auth;

    let mut ensure_mailboxes_exist: HashMap<String, MboxCheck> = HashMap::new();

    // If DKIM Authenticated messages are to be moved - make sure the associated Mailbox exists
    match &config.dkim_authenticated_move {
        Some(auth_outbox) => {
            if !do_dkim_auth {
                warn!("mode_dkim_auth was set to false - Turning to true due to dkim_authenticated_move set.");
                do_dkim_auth = true;
            }
            info!("Will move DKIM Authenticated emails to mailbox {} - Checking the existence.", &auth_outbox);
            ensure_mailboxes_exist.insert(auth_outbox.clone(), MboxCheck::default() );
        }
        None => info!("config.dkim_athenticated_move was not set - Will not move any DKIM Authenticated emails."),
    }

    // If DKIM Non-Authenticated messages are to be moved - make sure the associated Mailbox exists
    match &config.dkim_unauthenticated_move {
        Some(unauth_outbox) => {
            if !do_dkim_auth {
                warn!("mode_dkim_auth was set to false - Turning to true due to dkim_unauthenticated_move set.");
                do_dkim_auth = true;
            }
            info!("Will move DKIM Non-Authenticated emails to mailbox {} - Checking the existence.", &unauth_outbox);
            ensure_mailboxes_exist.insert(unauth_outbox.clone(), MboxCheck::default() );
        }
        None => info!("config.dkim_unathenticated_move was not set - Will not move any DKIM Non-Authenticated emails."),
    }

    if !ensure_mailboxes_exist.is_empty() {
        let mut list = fetch_session.list(Some("*"), Some("*")).await.unwrap();

        while let Some(item) = list.next().await {
            let saw = item.expect("Failed to fetch Mailbox list");
            let saw_name = saw.name();
            if let Some(ref mut mbox) = &mut ensure_mailboxes_exist.get_mut(saw_name) {
                mbox.exists = true;
            }
        }
        drop(list);

        for (create_mailbox, check_status) in ensure_mailboxes_exist.iter() {
            if !check_status.exists {
                info!("Creating needed mailbox {}", &create_mailbox);
                fetch_session.create(&create_mailbox).await?;
            } else {
                info!("Mailbox already exists {}", &create_mailbox);
            }
        }
    }

    Ok(do_dkim_auth)
}

// idle connection may spit out irrelevant notifications we will ignore
// re-calculate the new idle time based on duration if needed
pub(crate) fn calculate_idle_left(before: SystemTime, idle_secs_setting: u64) -> u64 {
    match before.elapsed() {
        Ok(elapsed) => {
            if elapsed.as_secs() >= idle_secs_setting {
                60 * 5
            } else {
                idle_secs_setting - elapsed.as_secs()
            }
        }
        Err(e) => {
            warn!("System clock error? {:?}", e);
            60 * 5
        }
    }
}

// idle connection may send responses that not interesting to us
// typically we only care about Maibox updates on the mailbox
// we are interested on
pub(crate) fn is_idle_response_interesting(
    idle_res: &IdleResponse,
    interesting_mailbox: &str,
) -> bool {
    match idle_res {
        IdleResponse::NewData(data) => {
            let parsed = data.parsed();
            match parsed {
                ImapResponse::MailboxData(mailbox_data) => match mailbox_data {
                    MailboxDatum::Status { mailbox, status } => {
                        if *mailbox == interesting_mailbox {
                            info!(
                                "Mailbox update on interested mailbox {:?} = {:?}",
                                mailbox, status
                            );
                            true
                        } else {
                            debug!(
                                "Mailbox Update on non-interested mailbox {:?} = {:?}",
                                mailbox, status
                            );
                            false
                        }
                    }
                    _ => {
                        debug!("MailboxData = {:?}", mailbox_data);
                        false
                    }
                },
                _ => {
                    debug!("NewData/Other = {:?}", parsed);
                    false
                }
            }
        }
        IdleResponse::Timeout => true,
        IdleResponse::ManualInterrupt => true,
    }
}
