# Fluvio IMAP Connector
Fluvio community Internet Message Access Protocol (IMAP) connector

## Source Connector
Reads from IMAP and writes to Fluvio topic.

### Configuration
| Option              | default  | type           | description                                                                                                    |
|:--------------------|:---------|:---------      |:---------------------------------------------------------------------------------------------------------------|
| host                | -        | String         | IMAP server                                                                                                    |
| port                | -        | Number         | IMAP server port - 143 for plaintext, 993 with TLS                                                             |
| user                | -        | String         | Username for plaintext login - must be over TLS - e.g. STARTTLS over 143 or directly over 993 TLS port         |
| password            | -        | String         | Password for plaintext login - must be over TLS                                                                |
| mailbox             | -        | String         | Mailbox to SELECT e.g. INBOX, Junk Mail etc. - deploy connector instance per Mailbox streaming                 |
| search              | -        | String         | e.g. UNSEEN - see RFC for SEARCH - this is executed upon new mail                                              |
| fetch               | -        | String         | e.g. (UID FLAGS INTERNALDATE RFC822.SIZE RFC822 RFC822.HEADER ENVELOPE BODYSTRUCTURE)                          |
| mode_bytes          | false    | bool           | Output bytes e.g. for headers & body RFC822 case                                                               |
| mode_utf8_lossy     | false    | bool           | Output lossy UTF8  - assume only into UTF8 Strings and scrap bytes                                             |
| mode_parser         | false    | bool           | Output parsed E-mail                                                                                           |
| mode_dkim_auth      | false    | bool           | Output status of Authentication-Results dkim method pass or fail                                               |
| dkim_authenticated_move | -    | String         | If the fetched new email has "Pass" status for Authentication-Results in DKIM method, move email into this Mailbox |
| dkim_unauthenticated_move | -  | String         | If the fetched new email has "Fail" or "None" status instead in the DKIM method, move email into this Mailbox  |
| dangerous_cert      | false    | String         | DANGEROUS: Upon development / debugging skip TLS cert verify - true (default false)                            |

Enable either mode_bytes or mode_utf8_lossy or both.

Various SASL authentication schemes can be implemented if needed, let us know in issues if one doesn't exist already.

### Usage Example

See [config-example.yaml](config-example.yaml) for an example reflecting the above.

Run connector locally using `cdk` tool (from root directory or any sub-directory):
```bash
fluvio install cdk

cdk deploy start --config config-example.yaml

cdk deploy list # to see the status
cdk deploy log my-imap-connector # to see connector's logs
```

## DKIM Authentication

This connector supports checking the "Authentication-Results" header (as per RFC 8601) to move the e-mail accordingly.

DKIM Non-Authenticated e-mails typically show up as:
```json
{"uid":"29","dkim_authenticated":false,"moved_to":"Unauthenticated","internaldate":"2024-07-05T02:26:27+00:00"}
```

DKIM Authenticated e-mails typically show up as:
```json
{"uid":"30","dkim_authenticated":true,"moved_to":"Authenticated","internaldate":"2024-07-05T02:26:27+00:00"}
```

### Notes

* DKIM Authentication relies on the e-mail infrastructure correctly handling "Message Authentication Status" via Authentication-Results header to set the dkim accordingly.

### Transformations
Fluvio Imap Source Connector supports [Transformations](https://www.fluvio.io/docs/concepts/transformations-chain/).

Records can be modified before sending to Fluvio topic.

## License
 
- * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 
### Contribution
 
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.



