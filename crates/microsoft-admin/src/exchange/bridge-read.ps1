function Test-ZixTrustee([string]$identity,[string]$trustee,[string]$address,$cache) {
    if ($identity -in @('NT AUTHORITY\SELF','S-1-5-10')) { return $false }
    if (!$identity) { throw 'permission_identity_missing' }
    if ($identity.ToLowerInvariant() -eq $address.ToLowerInvariant() -or $identity.ToLowerInvariant() -eq $trustee) { return $true }
    if (!$cache.ContainsKey($identity)) {
        $resolved = @(Get-EXORecipient -Identity $identity -Properties ExternalDirectoryObjectId -ErrorAction Stop)
        if ($resolved.Count -ne 1 -or !"$($resolved[0].ExternalDirectoryObjectId)") { throw 'permission_identity_ambiguous' }
        $cache[$identity] = "$($resolved[0].ExternalDirectoryObjectId)".ToLowerInvariant()
    }
    return $cache[$identity] -eq $trustee
}
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
        $rawKind = "$($m.RecipientTypeDetails)"
        if (!$id -and $primary -and $rawKind -eq 'DiscoveryMailbox') { $id = 'exchange-system:' + $primary }
        if (!$id -or !$primary) { throw 'incomplete_recipient' }
        $aliases = @($m.EmailAddresses | Where-Object { "$_" -match '^smtp:' } | ForEach-Object { ("$_").Substring(5).ToLowerInvariant() } | Sort-Object -Unique)
        $kind = if ("$($m.RecipientTypeDetails)" -in @('SharedMailbox','UserMailbox','DiscoveryMailbox')) { "$($m.RecipientTypeDetails)" } else { 'Other' }
        $full = $false; $send = $false
        if ($trustee -and $id -eq $target) {
            $delegate = @(Get-EXORecipient -ExternalDirectoryObjectId $trustee -Properties ExternalDirectoryObjectId,PrimarySmtpAddress -ErrorAction Stop)
            if ($delegate.Count -ne 1 -or "$($delegate[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $trustee -or !"$($delegate[0].PrimarySmtpAddress)") { throw 'trustee_mismatch' }
            $delegateAddress = "$($delegate[0].PrimarySmtpAddress)"
            # Enumerate first: filtering the cmdlet by an absent trustee can throw.
            # Resolve returned identities, rather than treating an exception as absence.
            $permissions = @(Get-EXOMailboxPermission -ExternalDirectoryObjectId $id -ResultSize 1001 -ErrorAction Stop)
            $sendPermissions = @(Get-EXORecipientPermission -ExternalDirectoryObjectId $id -ResultSize 1001 -ErrorAction Stop)
            if ($permissions.Count -gt 1000 -or $sendPermissions.Count -gt 1000) { throw 'permission_observation_limit' }
            $cache = @{}
            $selectedFull = @($permissions | Where-Object { Test-ZixTrustee "$( $_.User )" $trustee $delegateAddress $cache })
            $selectedSend = @($sendPermissions | Where-Object { Test-ZixTrustee "$( $_.Trustee )" $trustee $delegateAddress $cache })
            if (@($selectedFull | Where-Object { $_.Deny -or $_.IsInherited }).Count -gt 0) { throw 'ambiguous_permission' }
            if (@($selectedSend | Where-Object { $_.AccessControlType -ne 'Allow' -or $_.IsInherited }).Count -gt 0) { throw 'ambiguous_send_permission' }
            $full = @($selectedFull | Where-Object { $_.AccessRights -contains 'FullAccess' }).Count -gt 0
            $send = @($selectedSend | Where-Object { $_.AccessRights -contains 'SendAs' }).Count -gt 0
        }
        $rows += @{ id=$id; display_name="$($m.DisplayName)"; kind=$kind; primary=$primary; aliases=$aliases; full_access=$full; send_as=$send; sign_in_blocked=$null }
    }
    @{ scope=$r.scope; observed_at=[DateTimeOffset]::UtcNow.ToUnixTimeSeconds(); accepted_domains=$domains; recipients=$rows; complete=($all.Count -le 200 -and $allDomains.Count -le 100); trustee=$trustee }
}
