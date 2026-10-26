# Microsoft administration boundary

Rust library for Microsoft Graph discovery and explicit Exchange recipient changes.
No provider login, app registration, consent, DNS change or mail server is enabled
by importing or building it. Caller-owned tenant and subject UUIDs are mandatory.

## Supported operations

- Graph: organization, subscribed licenses/service plans, verified domains,
  domain mail-service DNS requirements, directory users, proxy addresses and
  per-user license/service-plan assignments.
- Exchange: bounded recipient snapshots, create shared mailbox, add/remove a
  secondary address, grant/revoke FullAccess and SendAs separately.
- Directory users/addresses are not evidence of an Exchange mailbox or active license.
- Human-reviewed, short-lived change plans bind exact state and tenant/subject.
  The CLI additionally binds plans to executable/catalog/version using the existing
  `zix-local-operation/v1` contract and operator-write terminal approval.

## Safety and execution

Graph uses the Crowsi HTTPS read transport: verified TLS, no proxy inheritance,
redirects or automatic retries. Each operation confirms `/me` and the selected
organization, then processes at most four pages / 200 items, eight requests and
2 MiB of aggregate response bytes and 96 KiB of projected row data. Incomplete/denied observations stay explicit.
Only declared fields are returned; reports can still contain private directory
information and must be stored privately, not in public Issues or repositories.

Exchange uses the official ExchangeOnlineManagement 3.10.1 module through an
owner-selected, absolute-path PowerShell profile. The module/runtime are external
prerequisites, never automatically installed. Only fixed cmdlets receive typed data;
there is no caller-supplied script, command name or undocumented Exchange REST API.
Graph and Exchange bearer credentials are separate, transient caller inputs.
Crowsi uses an explicit systemd user-service backend with strict 512 MiB /
32-task / one-CPU / 64 MiB per-file limits, zero swap and disabled core dumps.
All limits are applied before PowerShell starts; service creation failure never
falls back to an unbounded process. DOTNET_PROCESSOR_COUNT=1 bounds runtime thread
selection. The file cap accommodates .NET executable memfd mappings; the former
8 MiB cap prevented the official runtime from starting. A service has a 30-second
lifetime and one-second stop timeout, including after host death; immediate
parent-death teardown is not claimed. The full native request is bounded to 120 seconds;
a slow write can therefore end with an unknown outcome and must not be resent.

The journal must be a private, owner-provisioned canonical directory. An fsynced
per-plan attempt marker is created before invoking a mutation, plus a tenant lock.
A process crash, timeout, failed write response or failed post-read never authorizes
replay. This is one adapter-level cmdlet invocation, not a guarantee of exactly-once
HTTP execution inside Microsoft's module/service. Shared mailbox internal names
are deterministically derived from the approved address to avoid creating a
different recipient on an SDK retry. Only verified post-state releases the tenant lock. A later operator must
inspect the real state before manually resolving an uncertain lock; do not delete
markers or generate replacement plans merely to repeat a write.

Post-state must match the expected delta and preserve other observed recipients,
addresses, domains and selected explicit permissions. Removal of one explicit
grant does not prove the absence of group-based or inherited effective access. Shared creation additionally requires
Graph evidence that direct sign-in is blocked. Unknown sign-in state is not success.
Snapshots are bounded, not a transaction or a global lock against other admins.
Tenant-side concurrent changes can still require manual reconciliation.

## Boundaries

This version does not migrate historical messages, enable authentication methods,
change licenses, add/verify tenant domains, alter MX/SPF/DKIM/DMARC, send messages,
implement forms, or create application/tenant-wide mail permissions. Existing mail
packages remain separate. Actual resource/permission changes require their own
concrete approval. No billing amount or invoice is inferred from subscribed SKUs.

## Verification

`cargo test --workspace --all-targets --all-features` tests synthetic transport,
planning, drift, approval, persistent one-use markers and ambiguous outcomes.
`pwsh -NoProfile -File tests/exchange-bridge.ps1` exercises literal command mapping
and parses the production bridge without importing Exchange or authenticating.
Real Microsoft tenant, module compatibility, consent and strict child containment
acceptance remain separately required before live use; synthetic tests are not that
acceptance and no skipped real-account test is a success.

See [mail administration](../../docs/mail-administration.md) for explicit auth,
Graph mutations, DKIM, probes, runtime acceptance and remaining owner prerequisites.
