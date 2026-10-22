function Write-ZixRecipient($c) {
    # Only reviewed cmdlets with splatted literal data. No Invoke-Expression, scripts or caller command names.
    switch ($c.operation) {
        'create_shared' { New-Mailbox -Shared -Name $c.display_name -DisplayName $c.display_name -PrimarySmtpAddress $c.address -ErrorAction Stop | Out-Null }
        'add_alias' { Set-Mailbox -Identity $c.mailbox -EmailAddresses @{Add=$c.address} -ErrorAction Stop }
        'remove_alias' { Set-Mailbox -Identity $c.mailbox -EmailAddresses @{Remove=$c.address} -ErrorAction Stop }
        'grant_full_access' { Add-MailboxPermission -Identity $c.mailbox -User $c.trustee -AccessRights FullAccess -InheritanceType All -AutoMapping:$false -ErrorAction Stop | Out-Null }
        'revoke_full_access' { Remove-MailboxPermission -Identity $c.mailbox -User $c.trustee -AccessRights FullAccess -InheritanceType All -Confirm:$false -ErrorAction Stop }
        'grant_send_as' { Add-RecipientPermission -Identity $c.mailbox -Trustee $c.trustee -AccessRights SendAs -Confirm:$false -ErrorAction Stop | Out-Null }
        'revoke_send_as' { Remove-RecipientPermission -Identity $c.mailbox -Trustee $c.trustee -AccessRights SendAs -Confirm:$false -ErrorAction Stop }
        default { throw 'unknown_operation' }
    }
    @{ acknowledged=$true }
}
