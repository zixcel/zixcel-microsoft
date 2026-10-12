# Microsoft integration contracts

This repository provides credential-free Microsoft 365 planning contracts in Rust,
a display-only mailbox descriptor, and a separately installable browser mail client.
The host application owns account selection, consent, token custody and activation.

| Artifact | Purpose | Distribution |
| --- | --- | --- |
| `zixcel-microsoft` | Validate bounded acquisition requests and produce read-only plans | GitHub source; Cargo registry publishing disabled |
| `@zixcel/microsoft-mail-presentation` | Present configuration fields and missing prerequisites | npm |
| `@zixcel/microsoft-mail-web` | Read delegated Graph mail changes and perform explicitly consented actions | npm |

## Display a connection plan

```sh
npm install @zixcel/microsoft-mail-presentation@0.10.0
```

```ts
import { mailPresentation } from '@zixcel/microsoft-mail-presentation'
console.log(mailPresentation.label, mailPresentation.state)
for (const prerequisite of mailPresentation.requirements) {
  console.log(prerequisite.label, prerequisite.detail)
}
```

The descriptor has `PlanningOnly` status. It contains no credentials, performs no
network operations and does not validate or authorize a user's account. Its schema
is data for a host-owned form. Installing this package does not activate a service.

## Plan bounded read-only acquisition in Rust

```sh
cargo run --locked --offline -- mail-schema
cargo run --locked --offline -- mail-plan examples/mail-request.json
```

Supply opaque account and authorization references. Never put tokens, passwords or
client secrets in a request. Metadata requires delegated `Mail.ReadBasic`; bodies
separately require `Mail.Read`. Read-only plans do not request `Mail.ReadWrite` or
`Mail.Send`. Pages are bounded to 1–100 messages and responses to 1 MiB. Folder IDs
must be encoded as one path segment and requests use immutable message IDs.

Graph bodies are untrusted input. Acquisition, rendering, semantic interpretation
and authority to modify mail remain separate responsibilities. The planning crate
performs no networking, login, sending, moving or read-state changes. A successful
OIDC login does not establish Graph mail permission.

## Develop and verify

```sh
pnpm install --frozen-lockfile --ignore-scripts
pnpm test
pnpm typecheck
pnpm build
pnpm --filter @zixcel/microsoft-mail-web test
pnpm --filter @zixcel/microsoft-mail-web typecheck
```

Tests use reserved example addresses, a never-issued registration identifier and a
stub token. They do not access an account. Live Entra registration, account consent
and end-to-end host activation require separate integration verification.

[Browser client](packages/zixcel-microsoft-mail-web/README.md) ·
[Interface reference](docs/interface-reference.md) · [Security](SECURITY.md) ·
[License](LICENSE) · [Notices](NOTICE)

## Local administrator runtime

See [the separate Node provider package](packages/provider-microsoft/README.md)
for account-bound read/write operations and isolated fake-HTTP tests.
The existing Rust planning API remains independently scoped.
