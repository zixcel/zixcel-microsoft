# @zixcel/microsoft-mail-presentation

Check a Microsoft 365 integration configuration and produce a plan before implementing or enabling the connection.

## What you can do

- Validate the declared integration settings.
- Inspect a bounded plan and missing prerequisites.

## Current scope

The current package validates configuration and plans operations. It does not provide a complete live service client.

Package distribution is not activated by this documentation. Use the checked-in source and the declared dependency versions; published availability must be verified separately.

## Getting started

Install Rust 1.97 or newer and make the declared dependencies available. Use the configured private registry when a dependency is not distributed publicly. Run from this repository:

```sh
npm install
```

## Examples and interface details

## Generic planning commands

```bash
cargo run --offline -- doctor
cargo run --offline -- capabilities
cargo run --offline -- validate examples/config.toml
cargo run --offline -- plan examples/config.toml
cargo run --offline -- plan-authorized \
  examples/config.toml examples/provider-plan-request.json
```

Configuration contains only opaque tenant references and `secret://...` references. Tokens, passwords and client secrets are rejected. Generic `plan` describes future SharePoint, Outlook, Teams and OneDrive observations without communication or execution.

## Mail input contract (0.10.0)

`mail-plan` is an independent read-only acquisition plan. It performs no networking, login, continuation-page retrieval, classification, read-state modification, moving or sending.

```bash
cargo run --locked --offline -- mail-schema
cargo run --locked --offline -- mail-plan examples/mail-request.json
```

`schemas/mail-request-v1.schema.json` is a closed configuration contract suitable for UI projection. Accounts and authorization are references; actual candidates come from the credential owner. Never expose secrets to UI or treat free-form references as authenticated. Select mailboxes from authorized account enumerations. This package owns neither sem-lang semantics nor HAT/Role definitions.

- Metadata uses delegated `Mail.ReadBasic`, without bodies.
- Bodies separately require delegated `Mail.Read`; neither `Mail.ReadWrite` nor `Mail.Send` is requested.
- Every account explicitly supplies `account_ref` and `authorization_ref`, bound to the plan digest.
- Pages contain 1-100 messages; responses are limited to 1 MiB, with no arbitrary hosts or redirects.
- Encode folder IDs as one URL path segment; require ImmutableId.
- Bodies are untrusted external data. Display, semantic interpretation and effect authority are separate from acquisition.

Existing OIDC login is identity-only using `openid/profile/email`. Success does not imply Graph mail permission. Graph consent, token custody, Crowsi transfer, response verification and cursor retention still require host integration. Real mail availability in the Hatter product UI is not established at this stage.

References: [Graph messages](https://learn.microsoft.com/en-us/graph/api/user-list-messages?view=graph-rest-1.0) and [Immutable ID](https://learn.microsoft.com/en-us/graph/outlook-immutable-id).

## Documentation and source

[Interface reference](docs/interface-reference.md)

[Usage guide](docs/getting-started.md)

[Examples](examples) · [Schemas](schemas) · [Implementation and public interfaces](src) · [Verification cases](tests) · [Contributing](CONTRIBUTING.md) · [Security reporting](SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)
