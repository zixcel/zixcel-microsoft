function Write-ZixRecipient($c) {
    if ($c.operation -notin @('create_shared','add_alias','remove_alias','grant_full_access','revoke_full_access','grant_send_as','revoke_send_as')) { throw 'unknown_operation' }
    # Only reviewed cmdlets with splatted literal data. No Invoke-Expression, scripts or caller command names.
    $name = if ($c.operation -eq 'create_shared') { 'shared-' + [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($c.address))).Substring(0,20).ToLowerInvariant() } else { $null }
    $identity = $null; $trustee = $null
    if ($c.operation -ne 'create_shared') {
        $target = @(Get-EXOMailbox -ExternalDirectoryObjectId $c.mailbox -Properties ExchangeGuid,ExternalDirectoryObjectId -ErrorAction Stop)
        if ($target.Count -ne 1 -or "$($target[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $c.mailbox) { throw 'target_mismatch' }
        $identity = "$($target[0].ExchangeGuid)"
        if (!$identity) { throw 'exchange_identity_missing' }
    }
    if ($c.PSObject.Properties['trustee']) {
        $delegate = @(Get-EXORecipient -ExternalDirectoryObjectId $c.trustee -Properties ExternalDirectoryObjectId,PrimarySmtpAddress -ErrorAction Stop)
        if ($delegate.Count -ne 1 -or "$($delegate[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $c.trustee) { throw 'trustee_mismatch' }
        $trustee = "$($delegate[0].PrimarySmtpAddress)"
        if (!$trustee) { throw 'trustee_address_missing' }
    }
    switch ($c.operation) {
        'create_shared' { New-Mailbox -Shared -Name $name -Alias $name -DisplayName $c.display_name -PrimarySmtpAddress $c.address -ErrorAction Stop | Out-Null }
        'add_alias' { Set-Mailbox -Identity $identity -EmailAddresses @{Add=$c.address} -ErrorAction Stop }
        'remove_alias' { Set-Mailbox -Identity $identity -EmailAddresses @{Remove=$c.address} -ErrorAction Stop }
        'grant_full_access' { Add-MailboxPermission -Identity $identity -User $trustee -AccessRights FullAccess -InheritanceType All -AutoMapping:$false -ErrorAction Stop | Out-Null }
        'revoke_full_access' { Remove-MailboxPermission -Identity $identity -User $trustee -AccessRights FullAccess -InheritanceType All -Confirm:$false -ErrorAction Stop }
        'grant_send_as' { Add-RecipientPermission -Identity $identity -Trustee $trustee -AccessRights SendAs -Confirm:$false -ErrorAction Stop | Out-Null }
        'revoke_send_as' { Remove-RecipientPermission -Identity $identity -Trustee $trustee -AccessRights SendAs -Confirm:$false -ErrorAction Stop }
        default { throw 'unknown_operation' }
    }
    @{ acknowledged=$true }
}
