# Microsoft Graph mail protocol

This small Rust library builds fixed-origin application-token and plain-text sendMail requests and validates bounded token responses. It owns no network client, credential storage, application registration, Exchange administration or form routing. It has no SMTP/MIME/TLS implementation.

The caller must supply a tenant, app and sender mailbox by GUID; the `/me` delegated-user path is not used. The HTTP adapter must enforce timeouts, response limits and manual redirect rejection, keep credentials out of logs, and never automatically replay an uncertain mail write. A Graph 202 response means acceptance, not inbox delivery.

Use mailbox-scoped Exchange Application RBAC for `Application Mail.Send`, without an independent unscoped Entra mail grant. Recipient allowlists belong to the caller; mailbox scope does not restrict recipients. `saveToSentItems=false` avoids requesting another stored copy; verify actual mailbox settings and retention.

This crate is distributed from an immutable Git revision alongside the existing Microsoft source packages. There is no new registry identity or API token. The supported initial machine credential is an application client secret; selection and provisioning of production credentials requires the operator's review. Human Graph Explorer/device-login tokens are not production machine credentials.
