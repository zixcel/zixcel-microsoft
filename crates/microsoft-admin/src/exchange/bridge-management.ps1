function Read-ZixDkim($c) {
    if ($c.operation -notin @('prepare_dkim','enable_dkim')) { throw 'unknown_management_operation' }
    $domain = "$($c.domain)"
    $accepted = @(Get-AcceptedDomain -Identity $domain -ErrorAction Stop)
    if ($accepted.Count -ne 1 -or "$($accepted[0].DomainName)".ToLowerInvariant() -ne $domain -or "$($accepted[0].DomainType)" -ne 'Authoritative') { throw 'authoritative_domain_required' }
    # Enumerate successfully before inferring absence. Exchange versions differ
    # in the error category raised by -Identity when no signing config exists.
    $all = @(Get-DkimSigningConfig -ErrorAction Stop)
    if ($all.Count -gt 100) { throw 'dkim_observation_limit' }
    $config = @($all | Where-Object { "$($_.Domain)".ToLowerInvariant() -eq $domain })
    if ($config.Count -eq 0) { return @{ target=$null; eligible=$true } }
    if ($config.Count -ne 1 -or "$($config[0].Domain)".ToLowerInvariant() -ne $domain -or $config[0].Enabled -isnot [bool]) { throw 'dkim_identity_mismatch' }
    @{ target=@{domain=$domain; enabled=$config[0].Enabled; selector1="$($config[0].Selector1CNAME)".ToLowerInvariant().TrimEnd('.'); selector2="$($config[0].Selector2CNAME)".ToLowerInvariant().TrimEnd('.')}; eligible=$true }
}
function Write-ZixDkim($c) {
    switch ($c.operation) {
        'prepare_dkim' { New-DkimSigningConfig -DomainName $c.domain -Enabled $false -KeySize 2048 -ErrorAction Stop | Out-Null }
        'enable_dkim' { Set-DkimSigningConfig -Identity $c.domain -Enabled $true -ErrorAction Stop | Out-Null }
        default { throw 'unknown_management_operation' }
    }
    @{acknowledged=$true}
}
