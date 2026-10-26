# Local mail administration

Native Microsoft 0.3.4 requires zix CLI 0.4.0. Failed management writes retain
code-only phase, reason, HTTP status and allowlisted provider error codes in
the receipt and permanent attempt journal. Remote messages, headers and arbitrary
local error strings are never included. Unknown outcomes still retain the tenant
lock and forbid replay; diagnostics do not authorize reconciliation or retries.
Existing attempts without diagnostics cannot be reconstructed from this update.
`domains list` also returns the nullable domain operation `state`, projecting
only `operation`, `status` and `lastActionDateTime`. A null state does not prove
that an earlier write failed or authorize a retry.
Device authorization accepts
the current `https://login.microsoft.com/device` page as well as the two
legacy Microsoft device-login URLs, each by exact match. Graph read transport budgets
are capped at 40 seconds within the provider's 120-second operation budget;
shorter caller budgets are preserved. The GNU Linux archive includes
Crowsi Secret Service custody; the static archive retains token-environment use
but rejects `auth` operations with `credential_store_not_in_build`. The core is
provider-neutral: it supplies multiple explicitly bound credentials and never
contains Microsoft OAuth or tenant logic.

## Runtime and owner-provisioned application

Run `python3 tools/runtime/install.py` to install the pinned official PowerShell
7.6.9 and ExchangeOnlineManagement 3.10.1 distributions outside the checkout.
Downloads are bounded and checked against the official PowerShell release SHA256
and the pinned Gallery package SHA512. The installer refuses replacement. Its
receipt establishes downloaded-byte integrity, not independent signature trust.
A running local systemd user manager with memory, CPU and task controllers is required.
The service has a 30-second hard lifetime plus a one-second stop timeout; immediate
host-death termination is not claimed. Only public paths/options are passed in
service arguments; credentials travel through private stdin. Runtime import is tested separately by `zix microsoft runtime inspect` under the
same Crowsi systemd user-service containment used for Exchange operations.

Create a private directory (`0700`) for profiles, working data, plans and journals.
Use absolute canonical paths. Runtime setup is explicit and contains no secrets:

```sh
zix microsoft runtime configure --pwsh /absolute/official/pwsh \
  --module /absolute/official/ExchangeOnlineManagement.psd1 \
  --directory /absolute/private/work --user admin@example.net \
  --output /absolute/private/exchange.json
zix microsoft runtime inspect --profile /absolute/private/exchange.json
```

The owner must provision a tenant-specific Entra **public client** registration,
enable public-client device authorization, and consent the delegated permissions.
No Microsoft first-party client ID is borrowed. Initial application registration
and consent are prerequisites, not silently created by this tool. Tenant ID,
client ID, actual sign-in UPN and roles must be supplied by the owner. Conditional
Access can prohibit device flow; this implementation does not bypass that policy.

Graph baseline identity verification requires `User.Read,Organization.Read.All`.
Add only the scopes required for the chosen operation: `Domain.Read.All` or
`Domain.ReadWrite.All`; `User.Read.All` or `User.ReadWrite.All` (and
`User.EnableDisableAccount.All` for account-enabled changes);
`LicenseAssignment.Read.All` or `LicenseAssignment.ReadWrite.All`; `Mail.Send`
plus `Mail.Read` for probes. Applicable Entra administrative roles and Exchange
RBAC are also required; granted OAuth scopes alone do not prove authorization.
For Exchange, configure the application's delegated Office 365 Exchange Online `Exchange.Manage` permission and
consent explicitly; authentication requests the Outlook resource `.default`.
No application-only certificate flow is implemented by these commands.

```sh
zix microsoft auth configure --tenant TENANT_UUID --client-id CLIENT_UUID \
  --collection default --graph-permissions User.Read,Organization.Read.All,Domain.ReadWrite.All \
  --output /absolute/private/auth.json
zix microsoft auth start --profile /absolute/private/auth.json --resource graph
# Complete the returned code at the returned Microsoft verification URL.
zix microsoft auth poll --profile /absolute/private/auth.json --resource graph
zix microsoft auth status --profile /absolute/private/auth.json --resource graph
zix microsoft auth refresh --profile /absolute/private/auth.json --resource graph
```

Polling is one bounded request per invocation and returns the required delay;
respect `poll_after_seconds`. Use the same operations with `--resource exchange`
for its separate resource token. Tokens, refresh tokens and pending device codes
stay in Crowsi; only metadata and the short-lived browser user code are returned.
Graph checks `/me` and the tenant organization before accepting tokens. Exchange
validates subject, tenant and UPN on use through its official module. Local logout
removes local caches; it does not revoke remote consent. Store locking is explicit.

Use core `zix credential run --collection default --bindings FILE` with reviewed
manifest/executable hashes. The private `zix-credential-bindings/v1` document has
one entry per token: reference returned by auth, `field: "access_token"`, the
resource host (`graph.microsoft.com` or `outlook.office365.com`) and respectively
`ZIX_MICROSOFT_ACCESS_TOKEN` or `ZIX_MICROSOFT_EXCHANGE_TOKEN`. Only selected
access-token fields are forwarded. Refresh explicitly when expired. For creating
a user, enroll an initial password separately through hidden-terminal
`zix credential set` and bind `ZIX_MICROSOFT_INITIAL_PASSWORD` to host
`graph.microsoft.com`; passwords never appear in a change or plan file.

## Tenant operations

Read paths are `organization get`, `licenses list`, `domains list`, `users list`,
`users inspect`, `domains verification-records` and `domains dns-records`.
Verification records are available before verification; service configuration
records require an already verified domain. DNS values are taken from Microsoft,
including tenant-specific MX and DKIM targets; never derive them from assumptions.

For each change, write a private JSON change file using exactly one operation:

| Group | Change operation and fields |
| --- | --- |
| domains | `add_domain` / `verify_domain`: `domain` |
| users | `create_user`: `upn,display_name,mail_nickname,usage_location,account_enabled` |
| users | `set_usage_location`: `user` (UUID), `usage_location` |
| users | `set_user_enabled`: `user` (UUID), `enabled` |
| licenses | `assign_license`: `user` (UUID), `sku` (UUID) |
| dkim | `prepare_dkim` / `enable_dkim`: `domain` |
| mail | `send_probe`: `recipient`, `correlation` (fresh UUID) |

```sh
umask 077
# Invoke through the reviewed credential bindings described above.
zix microsoft domains plan --tenant TENANT_UUID --subject SUBJECT_UUID \
  --change-file /absolute/private/change.json > /absolute/private/plan-result.json
jq '.result.plan' /absolute/private/plan-result.json > /absolute/private/plan.json
# Review .result.sha256 and the extracted plan, then approve that SHA256 in the terminal.
zix microsoft domains apply --tenant TENANT_UUID --subject SUBJECT_UUID \
  --plan-file /absolute/private/plan.json --approval-sha256 REVIEWED_SHA256 \
  --journal-directory /absolute/private/journal
```

Use the appropriate group in place of `domains`; DKIM also accepts
`--runtime-profile /absolute/private/exchange.json`. Plans expire after five
minutes and bind the executable, catalog, tenant, subject, requested change and
observed state. State drift, stale approvals, replays and missing inputs fail
before mutation. A durable journal records one write attempt. Failed or uncertain
outcomes keep the tenant lock for manual read-only reconciliation; never delete
a marker merely to retry. The official Exchange module may perform internal
retries; the adapter invokes one cmdlet and does not claim exactly-once HTTP.

The sequence is: add domain; publish its verification record using the DNS
provider; verify ownership without force takeover; create or select a user;
set its usage location; assign an already purchased available Exchange-capable
SKU; wait for mailbox provisioning and inspect; publish Microsoft service DNS;
prepare DKIM disabled; inspect actual selectors; publish both CNAMEs; enable DKIM.
DNS repairs and external records are separate ValueDomain operations with their
own reviewed plans. This implementation does not purchase subscriptions or
change federation, connectors, migration batches or historical mail.

Send probes use the signed-in user's own mailbox and a fixed non-sensitive body.
A `202` response or an observed Sent Items copy does not prove delivery. The
receipt explicitly reports `sent_copy_observed_delivery_unverified`. Use
`mail probe-inspect --tenant ... --subject ... --correlation UUID` in the intended
recipient's authenticated mailbox to observe the exact probe in Inbox, and send
an external probe with the exact subject `zix-mail-probe-CORRELATION_UUID` for the
opposite direction. Reply prefixes such as `Re:` do not match this exact probe. It inspects no message body;
Inbox absence is not proof of non-delivery (rules/junk/quarantine remain possible).

Tests use synthetic backends and mocked cmdlets. Successful offline runtime
import is separate from real tenant authentication, permission acceptance,
mailbox provisioning, DNS propagation and actual mail delivery.

## Primary API references

- [Device authorization protocol](https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-code)
- [Domain verification](https://learn.microsoft.com/en-us/graph/api/domain-verify?view=graph-rest-1.0)
- [User property permissions](https://learn.microsoft.com/en-us/graph/api/user-update?view=graph-rest-1.0)
- [Assign existing licenses](https://learn.microsoft.com/en-us/graph/api/user-assignlicense?view=graph-rest-1.0)
- [Exchange access-token connection](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/connect-exchangeonline?view=exchange-ps)
- [DKIM configuration](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/new-dkimsigningconfig?view=exchange-ps)

## Explicit resolution of an uncertain domain verification

`domains plan --change-file FILE` with the local-only descriptor
`{"operation":"resolve_verification","original_plan_file":"/absolute/original.json",
"journal_directory":"/absolute/journal"}` is a
read-only planning command. It accepts only a previous `verify_domain` plan whose
exact inner digest matches the active tenant lock and permanent attempt. The
original attempt must be at least ten minutes old. Fresh domain observation must
include an explicit null state or a failed operation; scheduled/in-progress and
missing state are refused. Absence of an audit log or a null state does **not**
establish that the original request failed.

`domains apply --plan-file FILE --approval-sha256 HASH
--journal-directory DIR` requires the core's human terminal authorization. It
checks fresh state, plan expiry and journal digests again, writes an immutable
resolution receipt, and archives the active lock. It performs no remote writes.
The receipt explicitly retains the original outcome as unknown. The original
attempt and plan remain intact and can never be replayed. This is an explicit
operator decision to release local admission, not an inference of remote failure
or authorization to retry. Any subsequent remote operation requires a separate
fresh plan, preconditions and human authorization. This mechanism does not apply
to mailbox creation, mail sending or other potentially non-idempotent writes.

## Mailbox address cutover and inspection

The typed Exchange change `set_primary_smtp` takes an existing mailbox UUID and
an address in an authoritative accepted domain. It refuses other-recipient
conflicts and unchanged addresses. Exchange Online uses `Set-Mailbox
-EmailAddresses`: the bridge preserves all existing SMTP, X500, SIP and other
proxy addresses, demotes the old primary and promotes/adds exactly the requested
primary. It never recreates the mailbox. Independent post-read requires the
same recipient identities, prior aliases and other observed state. Removing an
old secondary alias remains a separate operation after delivery is verified.

`exchange snapshot --mailbox UUID` reads all bounded mailbox permission and
SendAs entries, SendOnBehalf, forwarding and hidden/visible inbox rules, including
sender and keyword conditions and redirect/forward/delete actions. Read failures
are errors, never empty successful inventories. It does not claim effective
access, folder-level permission coverage, transport-rule coverage or external
delivery. Selected-trustee snapshots enumerate and resolve returned permission
identities; an absent entry is distinct from a failed or ambiguous read.
