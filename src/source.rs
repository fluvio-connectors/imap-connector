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
use fluvio_connector_common::tracing::{debug, info, trace, warn};
use fluvio_connector_common::Source;
use futures::{stream::LocalBoxStream, StreamExt};

use msg_auth_status::alloc_yes::{MessageAuthStatus, ReturnPathVerifier, ReturnPathVerifierStatus};

use std::collections::HashMap;

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
    mode_parser: bool,
    mode_dkim_auth: bool,
    dkim_authenticated_move: Option<String>,
    dkim_unauthenticated_move: Option<String>,
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
        let mode_parser = config.mode_parser;
        let mode_dkim_auth = config.mode_dkim_auth;
        let dkim_authenticated_move = config.dkim_authenticated_move.clone();
        let dkim_unauthenticated_move = config.dkim_unauthenticated_move.clone();
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
            mode_parser,
            mode_dkim_auth,
            dkim_authenticated_move,
            dkim_unauthenticated_move,
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

    // Figure out if the message passed DKIM auth for Return-Path
    let mut do_dkim_auth = config.mode_dkim_auth;

    #[derive(Debug, Default)]
    struct MboxCheck {
        exists: bool,
    }

    let mut ensure_mailboxes_exist: HashMap<String, MboxCheck> = HashMap::new();

    // If DKIM Authenticated messages are to be moved - make sure the associated Mailbox exists
    // The mailbox must be set in dkim_authenticated_move within the Configuration
    // Consequently if do_dkim_auth was not set this will be set true if to be moved
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
    // The mailbox must be set in dkim_unauthenticated_move within the Configuration
    // Consequently if do_dkim_auth was not set this will be set true if to be moved
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

    if ensure_mailboxes_exist.len() > 0 {
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

    let idle_inbox = idle_session.select(&config.mailbox).await?;
    let _fetch_inbox = fetch_session.select(&config.mailbox).await?;

    info!("IMAP Connecting to Mailbox {}", &config.mailbox);
    info!("IMAP idle_inbox cur = {:?}", idle_inbox);

    let mut idle_handle = idle_session.idle();

    loop {
        let search = fetch_session.uid_search(config.search.clone()).await?;

        let mut to_fetch = vec![];
        for search_item in &search {
            to_fetch.push(*search_item);
        }
        drop(search);

        let mut uid_moves: Vec<(String, String)> = vec![];

        for fetch_uid in &to_fetch {
            info!("Fetching UID {:?}", fetch_uid);
            let mut fetch_new = fetch_session
                .uid_fetch(fetch_uid.to_string(), config.fetch.clone())
                .await?;
            while let Some(item_u) = fetch_new.next().await {
                let mut rec = ImapEvent {
                    uid: fetch_uid.to_string(),
                    dkim_authenticated: None,
                    dkim_authenticated_error: None,
                    moved_to: None,
                    flags: None,
                    body: None,
                    body_utf8_lossy: None,
                    header_parsed: None,
                    body_parsed: None,
                    header: None,
                    header_utf8_lossy: None,
                    text: None,
                    envelope: None,
                    internaldate: None,
                    body_structure: None,
                };

                let item = &item_u.unwrap();

                if let Some(header) = item.header() {
                    if config.mode_parser {
                        let parsed = mail_parser::MessageParser::default().parse(header).unwrap();
                        rec.header_parsed = Some(parsed);
                    }
                    if do_dkim_auth {
                        let parsed = mail_parser::MessageParser::default().parse(header).unwrap();
                        let auth_status = MessageAuthStatus::from_mail_parser(&parsed).unwrap();
                        let verifier =
                            ReturnPathVerifier::from_alloc_yes(&auth_status, &parsed).unwrap();
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
                        let parsed = mail_parser::MessageParser::default().parse(*body).unwrap();
                        rec.body_parsed = Some(parsed);
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
                    match rec.dkim_authenticated {
                        Some(true) => {
                            rec.moved_to = Some(dkim_move_to.clone());
                        }
                        _ => {}
                    }
                }

                // Move the mail in case Unauthenticated destination folder is set
                // and dkim_authenticated == false
                if let Some(dkim_move_to) = &config.dkim_unauthenticated_move {
                    match rec.dkim_authenticated {
                        Some(false) => {
                            rec.moved_to = Some(dkim_move_to.clone());
                        }
                        _ => {}
                    }
                }

                // Move the mail between Mailboxes
                if let Some(ref move_to) = rec.moved_to {
                    uid_moves.push((rec.uid.clone(), move_to.clone()));
                }

                tx.send(rec.try_into()?).await?;
            }
        }

        for (move_uid, move_to) in uid_moves.iter() {
            fetch_session.uid_mv(move_uid, move_to).await?;
        }

        info!("IMAP Idling & Waiting for 5 minutes");

        idle_handle.init().await?;
        let (idle_fut, _ss) = idle_handle.wait_with_timeout(std::time::Duration::from_secs(60 * 5));

        let idle_res = idle_fut.await?;

        info!("IMAP idle_res = {:?}", idle_res);
    }
}
