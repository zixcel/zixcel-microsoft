# Native Microsoft administration

The independent native `zix-microsoft` binary implements the Zix provider protocol.
Use [Microsoft administration](../../docs/microsoft-administration.md) for required
permissions, trusted Exchange profile, exact commands and one-use plan execution.

Version 0.3.1 retains `users get` and adds organization/license/domain/user discovery,
domain DNS requirements and separate Exchange snapshot/plan/apply commands. Graph
read routes use the Crowsi transport; Exchange uses fixed official PowerShell cmdlets
and Crowsi-owned bounded child processes. Manifest v2 marks apply as operator-write.

No implicit login, module installation, permission grant, token fallback, DNS write,
mail send, registry publication or live-account acceptance occurs on import/build.
Secrets are supplied by the credential owner, never as arguments or configuration.
Directory entries do not prove Exchange mailboxes. Unknown write outcomes cannot be
replayed. The complete native operation is limited to 120 seconds; each Exchange child is limited to 30 seconds.

The original single-user API facts remain locked under `api/`. The additional admin
library's official interface references are in its README and the guide; no upstream
prose or schema is redistributed. Build with the existing declared Rust toolchain:
`cargo build --locked -p zixcel-microsoft-cli`. Package version, source/content hashes,
notices and the actual dispatcher must be checked before independent publication.

The SDK is pinned to accepted source (native SDK 0.3.1), which verifies
the dispatcher's manifest-v2 binding. Both direct protocol and actual dispatcher
regressions are required; a v1-shaped direct request does not establish v2 compatibility.

Identity preflight failures now distinguish authentication rejection (HTTP 401),
permission denial (HTTP 403), service/limit failures and transport failure without
printing credential values or remote response bodies.

Patch 0.2.2 reads polymorphic DNS requirements without selecting derived fields on the base server entity, and preserves HTTP failure classes in bounded observations. It also preserves read-only Exchange DiscoveryMailbox records without an Entra UUID while retaining all SMTP address-conflict checks. Such observation references never authorize a write.

Version 0.3.1 adds explicit device authorization, Crowsi token-cache custody and
refresh, domain add/verify, user create/usage-location/account enablement, existing
license assignment, DKIM prepare/enable, and own-mailbox send/receive probes.
See [mail enablement guide](../../docs/mail-administration.md). CLI 0.4.0 supplies
provider-neutral multi-secret bindings. GNU packages include custody; static musl
packages accept explicit token environments and reject unsupported auth commands.
