# Native Microsoft administration

The independent native `zix-microsoft` binary implements the Zix provider protocol.
Use [Microsoft administration](../../docs/microsoft-administration.md) for required
permissions, trusted Exchange profile, exact commands and one-use plan execution.

Version 0.2.0 retains `users get` and adds organization/license/domain/user discovery,
domain DNS requirements and separate Exchange snapshot/plan/apply commands. Graph
read routes use the Crowsi transport; Exchange uses fixed official PowerShell cmdlets
and Crowsi-owned bounded child processes. Manifest v2 marks apply as operator-write.

No implicit login, module installation, permission grant, token fallback, DNS write,
mail send, registry publication or live-account acceptance occurs on import/build.
Secrets are supplied by the credential owner, never as arguments or configuration.
Directory entries do not prove Exchange mailboxes. Unknown write outcomes cannot be
replayed. The complete native operation is limited to 40 seconds.

The original single-user API facts remain locked under `api/`. The additional admin
library's official interface references are in its README and the guide; no upstream
prose or schema is redistributed. Build with the existing declared Rust toolchain:
`cargo build --locked -p zixcel-microsoft-cli`. Package version, source/content hashes,
notices and the actual dispatcher must be checked before independent publication.
