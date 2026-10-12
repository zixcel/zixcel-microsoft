# Isolated microsoft operations playground

From `packages/provider-microsoft`, run `npm ci --ignore-scripts`, `npm test`, then `node ../../.github/scripts/provider-consumer.mjs`. The owner CI runs this gate and the OIDC publisher repeats it before publication.

The gate packs the real candidate, creates a separate public-registry consumer with empty npm configuration, checks the public types, and exercises read/write, denial, resource binding, credential echo rejection and this runnable public-import example. All accounts, credentials and HTTP state are synthetic. No cloud CLI, host keyring or provider account is accessed.

`demo.mjs` prepares and approves a bounded mutation, observes its fake state using a separately approved read, and refuses replay without reacquiring credentials. The injected fake transport returns the same local object for the sample read; this is not a provider-specific production read-back recipe. Production callers must select the real resource read operation and verify its returned facts independently.

Keep account/resource/action scope, source/package version, grant expiry and revocation, allowed extension catalog, read-back and ambiguous outcome handling in the owner application. Generated code receives an execute port, not approval or credential access. Required permissions are metadata, not a complete IAM observation. The first catalog does not implement complete IAM discovery, normalized invoices or continuous monitoring. Those are future features, not completed by these fixtures.

[日本語](README.ja.md)

The publisher derives the archive/version from the manifest. On retry, an existing version must have exactly the candidate integrity and provenance; it is reused without claiming a new OIDC publication. A mismatch fails closed. Bump the package version for payload changes; do not rewrite an existing release.
