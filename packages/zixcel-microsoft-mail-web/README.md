# @zixcel/microsoft-mail-web

Connect a browser mail interface to Microsoft 365 with explicit delegated consent.

## What you can do

- Read supported folders and incremental mailbox changes.
- Request additional consent for reply, archive and read-state actions.

## Current scope

The shared bounded transport is the official `@crowsi/provider-http-transport@0.10.2` npm release; no sibling transport source or TGZ is required.

The application owns its Entra registration and session. Network access uses the configured bounded transport; mailbox content is not package data.

Install the browser package independently from npm:

```sh
npm install @zixcel/microsoft-mail-web@0.10.0
```

```ts
import { MicrosoftMailSource } from '@zixcel/microsoft-mail-web'
// The host supplies its public registration and dedicated redirect page.
const mail = new MicrosoftMailSource(clientId, 'organizations', redirectUri)
await mail.initialize()
await mail.connect()
const messages = await mail.list()
```

The redirect page is generated at build time with the typed
`@zixcel/microsoft-mail-web/redirect-document` export. It embeds only the installed
MSAL bridge. Authentication error details are not rendered into its document.
Account operations require explicit delegated consent; returned HTML mail bodies
remain untrusted and must be sanitized by the host before rendering.

## Getting started

Use the package manager matching the checked-in lockfile and the Node.js version declared in `package.json` or the development configuration. Run from this repository:

```sh
pnpm install --frozen-lockfile
pnpm test
```

## Documentation and source

[Usage guide](docs/getting-started.md)

[Implementation and public interfaces](src) · [Verification cases](test) · [Contributing](../../CONTRIBUTING.md) · [Security reporting](../../SECURITY.md) · [License](LICENSE) · [Attribution notices](NOTICE)

Metadata synchronization requests only delegated `Mail.ReadBasic`. Reading a message body requests `Mail.Read` separately through an explicit detail operation. An account switch invalidates in-flight body results before they can be returned. These boundaries supplement host authorization; mocked tests do not establish live Entra consent. See the [official delta permission table](https://learn.microsoft.com/en-us/graph/api/message-delta?view=graph-rest-1.0).
