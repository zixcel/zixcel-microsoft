# Microsoft 365 administration

The independent Rust admin library and native CLI own provider-specific operations.
No central release repository, HAT, semantic model or private company dataset is
required. Roles, tenant UUID, subject UUID, addresses and trusted runtime profiles
are supplied by the caller. No example contains usable credentials.

The resource-budget adapter is pinned to accepted public source bb2c7abb because
the existing crates.io 0.10.0 does not contain its budget API. The older published
crate remains in the shared CLI dependency graph; no duplicate process is started
by that fact. This package is not claimed to be an independently published crate.

## Read-only discovery

After independent installation/manifest verification and separately approved
Microsoft authentication/consent, provide a delegated Graph credential through the
existing process credential boundary (`ZIX_MICROSOFT_ACCESS_TOKEN`). It must never
appear in shell history, arguments, source, logs or public receipts. An identity-only
OIDC login does not grant the following permissions.

```sh
zix microsoft admin doctor
zix microsoft organization get --tenant TENANT_UUID --subject SUBJECT_UUID
zix microsoft licenses list --tenant TENANT_UUID --subject SUBJECT_UUID
zix microsoft domains list --tenant TENANT_UUID --subject SUBJECT_UUID
zix microsoft domains dns-records --tenant TENANT_UUID --subject SUBJECT_UUID --domain example.org
zix microsoft users list --tenant TENANT_UUID --subject SUBJECT_UUID
zix microsoft users inspect --tenant TENANT_UUID --subject SUBJECT_UUID --user USER_UUID
```

All new reads verify delegated `/me` and the selected organization. Required
permissions: `User.Read`, `Organization.Read.All`, then `Domain.Read.All` for domains,
`LicenseAssignment.Read.All` for subscriptions, `User.Read.All` for the selected user
properties. Supported Entra roles/admin consent are additionally required by
Microsoft. Declared permissions are not proof of effective access. Do not grant
`Directory.ReadWrite.All` or app-only mail rights just to make discovery succeed.

A domain added to Entra is not a mailbox. A proxy address is not an independent
mailbox. Exchange observations distinguish UserMailbox/SharedMailbox/other recipients.
Commercial SKU/service-plan status does not establish mailbox provisioning or costs.

## Exchange runtime and authentication

Owner supplies a private JSON profile via `ZIX_MICROSOFT_EXCHANGE_PROFILE`:

```json
{
  "pwsh": "/absolute/canonical/path/to/pwsh",
  "module_manifest": "/absolute/canonical/path/to/ExchangeOnlineManagement.psd1",
  "working_directory": "/absolute/private/exchange-runtime",
  "user_principal_name": "operator@example.org"
}
```

Provision this runtime only after installation approval. The working directory must be owner-only (0700); the profile must be private (0600).
The module must be the
reviewed official version 3.10.1; runtime code is trusted executable code, not LLM
configuration. Direct shared-mailbox login is not used. Supply a separate delegated
Exchange audience token through `ZIX_MICROSOFT_EXCHANGE_TOKEN`; a Graph token is not
an Exchange token. The connection's tenant and UPN plus its server-resolved subject
must match the selected scope. Secret custody/unlock belongs to Crowsi/owner ports;
this change neither enrolls credentials nor adds a second secret store.

## Typed changes and one-use execution

Save a private change file (mode 0600), for example:

```json
{"operation":"add_alias","mailbox":"33333333-3333-3333-3333-333333333333","address":"contact@example.org"}
```

```sh
zix microsoft exchange snapshot --tenant TENANT_UUID --subject SUBJECT_UUID
zix microsoft exchange plan --tenant TENANT_UUID --subject SUBJECT_UUID --change-file /absolute/private/change.json
```

Save the returned `result.plan` envelope alone, mode 0600. Review `result.sha256`
and every inner field. The plan is valid for two minutes, binds the current state,
and never creates an independent user/license or changes DNS implicitly.

```sh
zix microsoft exchange apply --tenant TENANT_UUID --subject SUBJECT_UUID --plan-file /absolute/private/plan.json --approval-sha256 EXACT_SHA256 --journal-directory /absolute/private/journal
```

The dispatcher requires a human terminal confirmation of this hash. Unlocking
custody is not approval. The provider rechecks executable/catalog, scope, age,
current recipients and domains before one write. Primary-address removal, duplicate
addresses, incomplete inventory and permission ambiguity are rejected. Alias updates
use add/remove deltas, never replacement of the entire address set.

Other change operations: `create_shared` (address/display_name), `remove_alias`
(mailbox/address), `grant_full_access`, `revoke_full_access`, `grant_send_as` and
`revoke_send_as` (mailbox/trustee UUIDs). Permission snapshots select that trustee
only; inherited/deny ambiguity stops the change. FullAccess does not imply SendAs.

`outcome_unknown` means inspect by read-only operations. Never resend the write,
remove a journal marker, or create a new plan to bypass an unresolved tenant lock.
Verified success describes the observed delta, not atomic exclusion of other admins.
New shared recipients must also be confirmed sign-in-blocked through Graph.

## Acceptance and official interfaces

Development tests use synthetic tenants and no real credentials. Live tenant reads,
Exchange module/runtime compatibility, credential consent/custody, mailbox changes,
actual email delivery and DNS migration are separate operational acceptance steps.

- [Graph domains](https://learn.microsoft.com/en-us/graph/api/domain-list)
- [Graph subscribed SKUs](https://learn.microsoft.com/en-us/graph/api/subscribedsku-list)
- [Graph user properties](https://learn.microsoft.com/en-us/graph/api/resources/user)
- [Exchange New-Mailbox](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/new-mailbox)
- [Exchange aliases](https://learn.microsoft.com/en-us/exchange/recipients-in-exchange-online/manage-user-mailboxes/add-or-remove-email-addresses)
