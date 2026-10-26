function Read-ZixMailboxAudit([string]$mailbox) {
    $target = @(Get-EXOMailbox -ExternalDirectoryObjectId $mailbox -Properties ExternalDirectoryObjectId,ExchangeGuid,EmailAddresses,ForwardingAddress,ForwardingSmtpAddress,DeliverToMailboxAndForward,GrantSendOnBehalfTo -ErrorAction Stop)
    if ($target.Count -ne 1 -or "$($target[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $mailbox) { throw 'target_mismatch' }
    $m = $target[0]; $identity = "$($m.ExchangeGuid)"
    if (!$identity) { throw 'exchange_identity_missing' }
    $full = @(Get-EXOMailboxPermission -ExternalDirectoryObjectId $mailbox -ResultSize 1001 -ErrorAction Stop)
    $send = @(Get-EXORecipientPermission -ExternalDirectoryObjectId $mailbox -ResultSize 1001 -ErrorAction Stop)
    $rules = @(Get-InboxRule -Mailbox $identity -IncludeHidden -ErrorAction Stop)
    if ($full.Count -gt 1000 -or $send.Count -gt 1000 -or $rules.Count -gt 100) { throw 'mailbox_audit_limit' }
    $fullRows = @($full | ForEach-Object { @{user="$($_.User)";rights=@($_.AccessRights | ForEach-Object {"$_"});deny=[bool]$_.Deny;inherited=[bool]$_.IsInherited} })
    $sendRows = @($send | ForEach-Object { @{user="$($_.Trustee)";rights=@($_.AccessRights | ForEach-Object {"$_"});access_type="$($_.AccessControlType)";inherited=[bool]$_.IsInherited} })
    $ruleRows = @($rules | ForEach-Object { @{identity="$($_.Identity)";name="$($_.Name)";enabled=[bool]$_.Enabled;priority=[int]$_.Priority;from=@($_.From | ForEach-Object {"$_"});subject_contains=@($_.SubjectContainsWords);body_contains=@($_.BodyContainsWords);subject_or_body_contains=@($_.SubjectOrBodyContainsWords);redirect_to=@($_.RedirectTo | ForEach-Object {"$_"});forward_to=@($_.ForwardTo | ForEach-Object {"$_"});delete=[bool]$_.DeleteMessage;stop_processing=[bool]$_.StopProcessingRules} })
    @{mailbox=$mailbox;read_only=$true;email_addresses=@($m.EmailAddresses|ForEach-Object {"$_"});full_access_entries=$fullRows;send_as_entries=$sendRows;send_on_behalf=@($m.GrantSendOnBehalfTo|ForEach-Object {"$_"});forwarding_address="$($m.ForwardingAddress)";forwarding_smtp="$($m.ForwardingSmtpAddress)";deliver_and_forward=[bool]$m.DeliverToMailboxAndForward;inbox_rules=$ruleRows;folder_permissions_observed=$false;transport_rules_observed=$false;effective_access_inferred=$false}
}
