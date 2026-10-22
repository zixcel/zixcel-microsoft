function Read-ZixRecipients($r) {
    $allDomains = @(Get-AcceptedDomain -ResultSize 101 -ErrorAction Stop)
    $domains = @($allDomains | Where-Object { $_.DomainType -eq 'Authoritative' } | ForEach-Object { "$($_.DomainName)".ToLowerInvariant() })
    $all = @(Get-EXORecipient -ResultSize 201 -Properties EmailAddresses,PrimarySmtpAddress,RecipientTypeDetails,ExternalDirectoryObjectId -ErrorAction Stop)
    $rows = @()
    $trustee = if ($r.change -and $r.change.PSObject.Properties['trustee']) { "$($r.change.trustee)" } else { $null }
    $target = if ($r.change -and $r.change.PSObject.Properties['mailbox']) { "$($r.change.mailbox)" } else { $null }
    foreach ($m in ($all | Select-Object -First 200)) {
        $id = "$($m.ExternalDirectoryObjectId)".ToLowerInvariant()
        $primary = "$($m.PrimarySmtpAddress)".ToLowerInvariant()
        if (!$id -or !$primary) { throw 'incomplete_recipient' }
        $aliases = @($m.EmailAddresses | Where-Object { "$_" -match '^smtp:' } | ForEach-Object { ("$_").Substring(5).ToLowerInvariant() } | Sort-Object -Unique)
        $kind = if ("$($m.RecipientTypeDetails)" -in @('SharedMailbox','UserMailbox')) { "$($m.RecipientTypeDetails)" } else { 'Other' }
        $full = $false; $send = $false
        if ($trustee -and $id -eq $target) {
            $delegate = @(Get-EXORecipient -ExternalDirectoryObjectId $trustee -Properties ExternalDirectoryObjectId,PrimarySmtpAddress -ErrorAction Stop)
            if ($delegate.Count -ne 1 -or "$($delegate[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $trustee -or !"$($delegate[0].PrimarySmtpAddress)") { throw 'trustee_mismatch' }
            $delegateAddress = "$($delegate[0].PrimarySmtpAddress)"
            $permissions = @(Get-EXOMailboxPermission -ExternalDirectoryObjectId $id -User $delegateAddress -ResultSize 21 -ErrorAction Stop)
            if ($permissions.Count -gt 20 -or @($permissions | Where-Object { $_.Deny -or $_.IsInherited }).Count -gt 0) { throw 'ambiguous_permission' }
            $full = @($permissions | Where-Object { $_.AccessRights -contains 'FullAccess' }).Count -gt 0
            $sendPermissions = @(Get-EXORecipientPermission -ExternalDirectoryObjectId $id -Trustee $delegateAddress -ResultSize 21 -ErrorAction Stop)
            if ($sendPermissions.Count -gt 20 -or @($sendPermissions | Where-Object { $_.AccessControlType -ne 'Allow' }).Count -gt 0) { throw 'ambiguous_send_permission' }
            $send = @($sendPermissions | Where-Object { $_.AccessRights -contains 'SendAs' }).Count -gt 0
        }
        $rows += @{ id=$id; display_name="$($m.DisplayName)"; kind=$kind; primary=$primary; aliases=$aliases; full_access=$full; send_as=$send; sign_in_blocked=$null }
    }
    @{ scope=$r.scope; observed_at=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds(); accepted_domains=$domains; recipients=$rows; complete=($all.Count -le 200 -and $allDomains.Count -le 100); trustee=$trustee }
}
