# zixcel-microsoft

An independent connector for Microsoft 365/Graph configuration validation and deterministic planning. It links no HTTP client, credential store or remote executor. This process makes no network calls, follows no redirects and resolves no credentials.

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

## SharePoint department daily reports

Report acquisition separates requests, plans, host execution and offline acceptance into distinct boundaries.

```text
versioned request
  -> deterministic plan
  -> separately authorized Host executor
  -> bounded local JSON artifact
  -> offline validation + SHA-256 receipt
```

Request validation and planning:

```bash
cargo run --offline -- sharepoint-daily-report validate-request \
  examples/sharepoint-daily-report-request-v1.json
cargo run --offline -- sharepoint-daily-report plan \
  examples/sharepoint-daily-report-request-v1.json
```

After a host executor places a SharePoint API response in a regular local JSON file, only that file is accepted offline. The following verifies a bundled fixture without communication.

```bash
cargo run --offline -- sharepoint-daily-report accept \
  examples/sharepoint-daily-report-request-v1.json \
  examples/department-daily-report-v1.json
```

Request v1 requires:

- `request_id`, `correlation_id`, `artifact_id`
- Opaque `site:...` and `drive:...` references
- Drive-root-relative `.json` paths without leading `/`, `.`, `..` or backslashes
- `expected_schema`: a bounded versioned caller-owned contract reference, such as `example://operations/department-daily-report/v1`
- Closed `expected_artifact` identifying headquarters, department, reporting team, classification and customer-data presence
- `media_type = application/json`
- `max_bytes` in `1..=1048576`

Plan IDs derive deterministically from the complete request and safety boundary. Host steps declare a future network read in a separate process, but `network_calls_performed`, `redirects_followed` and `credentials_resolved` are all `false` during planning. Hosts must reject redirects too.

`plan-authorized` accepts a Provider Plan Request with a closed Constillo receipt, independently recomputes its digest and returns a Microsoft plan. It carries only IDs/digests of the request, receipt and decision; network access, secret resolution and external actions are rejected. Contracts: [`provider-plan-request-v1.schema.json`](schemas/provider-plan-request-v1.schema.json) and [`authorized-connector-plan-v1.schema.json`](schemas/authorized-connector-plan-v1.schema.json).

`accept` rejects symlinks and non-regular files and reads no more than the request bound. It validates required Department Daily Report v1 fields, enums, identifiers, duplicates, dates, timestamps, source references, metrics, exceptions and summary limits. Headquarters, department, reporting team, classification and customer-data presence must match `expected_artifact`. Receipts record only the exact input SHA-256, byte count, opaque transport ID and verified downstream scope. SharePoint site, drive, path, artifact body and credentials are not copied into receipts or onto disk. Receipts certify acceptance of a host-supplied artifact, not a SharePoint connection.

Contracts are in [`schemas/`](schemas/README.md) and examples in [`examples/`](examples/).

This repository has no Cargo path dependency on other local repositories. Authorized host executors remain separate processes; do not link networking or secret-handling capabilities into this crate.

Library use separates generic `parse_config` / `build_plan` from `parse_daily_report_request`, `build_daily_report_plan` and `accept_daily_report_artifact`. Modules separate acquisition planning, artifact contracts, path safety, offline acceptance, content-free receipts and digests. The crate uses `publish = false` during local validation.

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

## Microsoft OIDC identity boundary

`build_oidc_login_plan` creates a closed Authorization Code + PKCE plan for Microsoft Entra. It validates a tenant-UUID-pinned issuer, public client ID, IPv4 loopback callback, `openid/profile/email` scopes only, independent state/nonce, S256 challenge and allowlisted authentication methods. This crate neither opens browsers nor follows redirects nor obtains, validates or stores tokens. Method selection becomes a guarantee only when enforced by host Entra policy; the plan alone is not authentication evidence.

## Quality gate

These checks run within this crate without connecting to Microsoft 365.

```bash
# WONDERLAND_ROOT is the workspace checkout root.
"$WONDERLAND_ROOT/bin/verify-repositories" --rust --tier standard
```

## Package integration

The package is an independently consumable unit. Callers reference its documented
interface through a versioned dependency and own application-specific composition
and integration.
