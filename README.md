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
| fetch               | -        | String         | e.g. (UID FLAGS INTERNALDATE RFC822.SIZE RFC822 RFC822.HEADER ENVELOPE BODYSTRUCTURE)                          |
| dangerous_cert      | false    | String         | DANGEROUS: Upon development / debugging skip TLS cert verify - true (default false)                            |

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

Insert records:
```bash
```

The produced record in Fluvio topic will be:
```json
{}
```

### Transformations
Fluvio Imap Source Connector supports [Transformations](https://www.fluvio.io/docs/concepts/transformations-chain/).

Records can be modified before sending to Fluvio topic.

## License
 
- * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
 
### Contribution
 
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.



