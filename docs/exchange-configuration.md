# Exchange configuration and routing (native 0.4.0)

The independent Microsoft extension adds these capabilities to the existing Zix
command routes. No additional root command or installer is required. Install the
verified official native release before operational use. Examples use synthetic
identities; save operational descriptors and plans privately with mode 0600.

## Read-only inventory

All Exchange examples require `--tenant TENANT_UUID --subject SUBJECT_UUID` and
`--runtime-profile /absolute/private/exchange-runtime.json`. Supply the Graph and
Exchange credentials through the existing Crowsi multi-secret bindings. Graph
Explorer tokens have the Graph audience and cannot replace Exchange tokens.

```sh
zix microsoft exchange snapshot --tenant TENANT_UUID --subject SUBJECT_UUID --runtime-profile /absolute/private/runtime.json --resource organization
zix microsoft exchange snapshot --tenant TENANT_UUID --subject SUBJECT_UUID --runtime-profile /absolute/private/runtime.json --resource policies --kind mobile
zix microsoft exchange snapshot --tenant TENANT_UUID --subject SUBJECT_UUID --runtime-profile /absolute/private/runtime.json --resource mailbox --mailbox MAILBOX_UUID
zix microsoft exchange snapshot --tenant TENANT_UUID --subject SUBJECT_UUID --runtime-profile /absolute/private/runtime.json --resource inbox_rules --mailbox MAILBOX_UUID
```

| Resource | Additional selector | Observed scope |
| --- | --- | --- |
| `organization` | none | SMTP AUTH default, audit switch, default authentication policy, accepted domains |
| `policies` | `--kind authentication/mobile/retention/sharing/owa/address_book/inbound_spam/outbound_spam/anti_phish` | Selected policy settings and default flags; anti-spam/phish rule bindings include priority, targets and exceptions |
| `mailbox` | `--mailbox UUID` | Sent As/On Behalf copies, GAL visibility, retention/sharing/address-book assignments |
| `client_access` | `--mailbox UUID` | OWA/MAPI/ActiveSync/POP/IMAP/SMTP AUTH and OWA/mobile assignments |
| `user_policy` | `--mailbox UUID` | Authentication policy assignment; null means organization default |
| `folders` | `--mailbox UUID` | Folder paths, IDs, types |
| `folder_permissions` | `--mailbox UUID --folder '\Inbox'` | Folder ACLs, including Default/Anonymous when present; use the actual localized path, with one leading backslash |
| `inbox_rules` | `--mailbox UUID` | Rules including hidden rules, supported conditions/actions, unsupported property names |
| `mobile_devices` | `--mailbox UUID` | Exchange device records and access state, not Intune compliance |
| `auto_mapping` | `--mailbox UUID --trustee UUID` | Explicit selected FullAccess and other grants; mapping itself is not observable |
| `transport_rules` | none | Enabled state, mode, priority, descriptions, conditions, exceptions and actions |
| `connectors` | none | Selected inbound/outbound connectors and TLS/routing settings |
| `remote_domains` | none | Automatic forwarding and selected remote-domain settings |
| `rbac` | none | Role groups and management role assignments/scopes |

These are bounded projections, not full configuration export or an effective-access
calculation. Sections cap at 200 items (Inbox rules: 100); routing traversal caps at
16 downstream addresses. Limits and failures are incomplete observations, never
empty success. The existing mailbox snapshot/audit remains available without
`--resource`, including explicit FullAccess, SendAs, forwarding and delegate facts.
Group expansion and folder ACLs must be assessed alongside it; administrators and
compliance access are separate from explicit mailbox delegation.

Graph-only directory/security inventory reuses `organization get`:

```sh
zix microsoft organization get --tenant TENANT_UUID --subject SUBJECT_UUID --resource groups
zix microsoft organization get --tenant TENANT_UUID --subject SUBJECT_UUID --resource group_members --identity GROUP_UUID
zix microsoft organization get --tenant TENANT_UUID --subject SUBJECT_UUID --resource user_memberships --identity USER_UUID
zix microsoft organization get --tenant TENANT_UUID --subject SUBJECT_UUID --resource security_defaults
zix microsoft organization get --tenant TENANT_UUID --subject SUBJECT_UUID --resource conditional_access
```

Group queries are transitive. Graph additionally requires appropriate delegated
`GroupMember.ReadBasic.All` or `GroupMember.Read.All` for member IDs/types
(group discovery may need `Group.Read.All`),
`User.Read.All` for another user's memberships, and `Policy.Read.All` for security
policy reads, alongside existing identity preflight permissions. Hidden membership
can additionally require `Member.Read.Hidden`; limited-information objects do not
prove absence of membership. Conditional Access availability also depends on
licensing and Entra roles. No permissions, licenses, policies or consent are added
automatically. Entra policy reads do not configure MFA or manage Intune.

Exchange authorization is delegated `Exchange.Manage` with suitable Exchange RBAC
for each fixed cmdlet; a successful connection alone is not sufficient. Read-only
roles may observe but cannot apply changes. No global admin role is implicitly
required or granted by this tool.

## Finite settings changes

Create a change descriptor such as:

```json
{"operation":"configure","target":{"resource":"mailbox","mailbox":"33333333-3333-3333-3333-333333333333"},"values":{"MessageCopyForSentAsEnabled":true,"MessageCopyForSendOnBehalfEnabled":true}}
```

Writable properties are deliberately finite:

- Mailbox: the two Sent Items copy switches, `HiddenFromAddressListsEnabled`,
  `RetentionPolicy`, `SharingPolicy`, `AddressBookPolicy`.
- Client access: `OWAEnabled`, `MAPIEnabled`, `ActiveSyncEnabled`, `PopEnabled`,
  `ImapEnabled`, `SmtpClientAuthenticationDisabled`, `OwaMailboxPolicy`,
  `ActiveSyncMailboxPolicy`. Null SMTP AUTH means inherit.
- User policy: `AuthenticationPolicy` (null removes the explicit assignment).
- Organization: `SmtpClientAuthenticationDisabled`.
- Policy kind `mobile`: `PasswordEnabled`, `MinPasswordLength` (4–16),
  `DeviceEncryptionEnabled`; `authentication`: `AllowBasicAuthPop/Imap/Smtp`;
  `outbound_spam`: `AutoForwardingMode` (`Off`, `On`, `Automatic`).
- Remote domains: `AutoForwardEnabled`.

Policy and remote-domain changes require `identity` with an exactly observed Name
or Identity. Wildcards and arbitrary property/cmdlet names are rejected. Other
policy types and transport rules/connectors are read-only in this release.
Authentication policy switches cannot restore protocols Microsoft has retired.
Hiding a mailbox from address lists can prevent delegated desktop Outlook SendAs;
GAL hiding is not an external-domain confidentiality control.

## Routing CRUD

Example: deliver a matching message from a common box to an internal child box:

```json
{
  "operation":"create_inbox_rule",
  "mailbox":"33333333-3333-3333-3333-333333333333",
  "acknowledge_outlook_rules_blob_removal":true,
  "rule":{
    "name":"zix:nursery",
    "priority":0,
    "from":["school@example.org"],
    "subject_contains":[],
    "body_contains":[],
    "subject_or_body_contains":["nursery","class"],
    "delivery":"redirect",
    "recipients":["child@example.org"],
    "keep_original":false,
    "stop_processing":true,
    "allow_external":false
  }
}
```

Different condition categories are AND; multiple entries in one category are OR.
`subject_or_body_contains` matches either field. Keywords are literal cmdlet
arguments, never PowerShell source. At least one condition is required. Recipient
addresses must be explicit; external destinations additionally require
`allow_external:true` and remain subject to tenant forwarding restrictions.
`redirect` preserves original sender semantics; `forward` produces forwarding
semantics. `keep_original:false` uses Exchange `DeleteMessage`, not permanent erase;
Deleted Items/retention may retain the original.

`update_inbox_rule` additionally requires the observed `identity` and a complete
`rule`; only an enabled, representable `zix:` rule can be updated.
`delete_inbox_rule` requires mailbox, identity and the acknowledgement.
`set_inbox_rule_enabled` adds `enabled:true/false` and optional `allow_external`.
Read the current rules before choosing the exact identity. The tool never edits
hidden or unsupported rules. Other rule content and relative order must survive;
Exchange may renumber absolute priorities.

Exchange rule mutation can remove Outlook client-only rule blobs; each CRUD/toggle
operation requires the explicit acknowledgement above, which is also disclosed in
the plan. Plans check mailbox forwarding and downstream Inbox rules for known
loops and refuse incomplete internal traversal. They cannot prove absence of loops
through transport rules or external services. Inspect those inventories separately.

## AutoMapping and mobile

```json
{"operation":"set_auto_mapping","mailbox":"33333333-3333-3333-3333-333333333333","trustee":"44444444-4444-4444-4444-444444444444","enabled":true}
```

This adds a missing direct FullAccess grant or removes and re-adds an exact existing
allow/non-inherited FullAccess grant with the selected AutoMapping flag. Existing
SendAs and other delegates must remain identical. Remove/re-add is not atomic; a
partial failure retains an unknown journal and may require a separate repair plan.
A verified ACL receipt is `permission_verified_client_check_required` because
permission listing cannot read the AutoMapping flag back. Outlook client
verification remains necessary. Outlook mobile requires adding each shared mailbox
inside the signed-in parent account; desktop AutoMapping does not perform this.

## Plan, confirm, apply, reconcile

```sh
zix microsoft exchange plan --tenant TENANT_UUID --subject SUBJECT_UUID --runtime-profile /absolute/private/runtime.json --change-file /absolute/private/change.json
zix microsoft exchange apply --tenant TENANT_UUID --subject SUBJECT_UUID --runtime-profile /absolute/private/runtime.json --plan-file /absolute/private/plan.json --approval-sha256 PLAN_SHA256 --journal-directory /absolute/private/journal --yes
```

Save only `result.plan` as the envelope. The SHA binds that exact envelope; CLI
0.5.0 accepts `y`/`yes` or `-y`/`--yes`, so no terminal/hash typing is required for
automation. New configuration plans expire after 10 minutes and bind complete
observed state. Unknown selectors/fields/types are rejected before credentials.
Current state is re-read before the write, with no mutation retry. Up to three
bounded post-reads verify the requested change and preservation of other observed
fields. Legacy mailbox plans retain their existing schema and expiry.

If the result is unknown, retain plans/journals and inspect without replaying.
After the original 10-minute window, a local-only resolution descriptor can be
planned through the same route:

```json
{"operation":"resolve_configuration","original_plan_file":"/absolute/private/original-plan.json","journal_directory":"/absolute/private/journal"}
```

Resolution requires an exact original, requested, or recognized AutoMapping partial
state, a fresh observation and matching attempt evidence. The original unknown
outcome and permanent replay ban remain. It archives the active tenant lock without
a remote write; unrelated drift blocks resolution. A new remote change always
requires a separate plan and approval.

## Official interfaces

- [New-InboxRule and client rule blob behavior](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/new-inboxrule)
- [Set-InboxRule](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/set-inboxrule)
- [Add-MailboxPermission AutoMapping](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/add-mailboxpermission)
- [Set-CASMailbox](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/set-casmailbox)
- [Folder permissions](https://learn.microsoft.com/en-us/powershell/module/exchangepowershell/get-exomailboxfolderpermission)
- [Transitive group members](https://learn.microsoft.com/en-us/graph/api/group-list-transitivemembers)
- [Conditional Access policies](https://learn.microsoft.com/en-us/graph/api/conditionalaccessroot-list-policies)
- [Security Defaults](https://learn.microsoft.com/en-us/graph/api/identitysecuritydefaultsenforcementpolicy-get)
- [Add shared mailbox in Outlook mobile](https://support.microsoft.com/en-us/outlook/add-a-shared-mailbox-to-outlook-mobile)
