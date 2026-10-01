# SharePoint daily-report contracts

The `sharepoint-daily-report-*-v1.schema.json` files define one closed boundary:

1. A consuming organization emits a versioned request containing only
   opaque SharePoint references and a drive-root-relative JSON path.
2. `zixcel-microsoft` creates a deterministic, non-executing plan.
3. A separately authorized Host executor resolves credentials and performs the
   remote read outside this process.
4. The Host stages the response as a local regular file.
5. `zixcel-microsoft` validates that file offline and emits a content-free
   SHA-256 receipt without the SharePoint source locator or report body.

The request and plan are not evidence that SharePoint was contacted. The
receipt proves only that the exact bounded local bytes were accepted under the
request contract. Its report scope and classification are copied only after
matching the closed `expected_artifact`; site, drive, path, body, and
credentials are not included. In every output from this process, network calls,
redirects, and credential resolution remain `false`.

`provider-plan-request-v1.schema.json` and
`authorized-connector-plan-v1.schema.json` define a separate local planning
boundary. It accepts an intact Constillo workflow receipt, rejects unknown
fields or changed digest material, and returns only provider-plan and upstream
evidence identifiers/digests. It does not grant a Host executor authority.
