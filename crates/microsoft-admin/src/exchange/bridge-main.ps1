$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$WarningPreference = 'SilentlyContinue'
$InformationPreference = 'SilentlyContinue'
Set-StrictMode -Version Latest
$r = $null
try {
    $chars = [char[]]::new(65537)
    $length = [Console]::In.ReadBlock($chars,0,$chars.Length)
    if ($length -gt 65536) { throw 'request_limit' }
    $inputText = [string]::new($chars,0,$length)
    [Array]::Clear($chars,0,$chars.Length)
    $r = ConvertFrom-Json -InputObject $inputText -Depth 10
    $inputText = $null
    if ($r.mode -notin @('snapshot','apply','management_snapshot','management_apply','mailbox_audit')) { throw 'unknown_mode' }
    Import-Module -Name $r.module -RequiredVersion '3.10.1' -ErrorAction Stop
    Connect-ExchangeOnline -AccessToken $r.token -UserPrincipalName $r.user -ShowBanner:$false -SkipLoadingCmdletHelp -ErrorAction Stop | Out-Null
    $connections = @(Get-ConnectionInformation | Where-Object { $_.State -eq 'Connected' })
    if ($connections.Count -ne 1 -or "$($connections[0].TenantID)".ToLowerInvariant() -ne $r.scope.tenant -or "$($connections[0].UserPrincipalName)".ToLowerInvariant() -ne $r.user) { throw 'connection_identity_mismatch' }
    $user = Get-User -Identity $r.user -ErrorAction Stop
    if ("$($user.ExternalDirectoryObjectId)".ToLowerInvariant() -ne $r.scope.subject) { throw 'subject_mismatch' }
    $result = switch ($r.mode) {
        'mailbox_audit' { Read-ZixMailboxAudit $r.audit_mailbox }
        'snapshot' { Read-ZixRecipients $r }
        'apply' { Write-ZixRecipient $r.change }
        'management_snapshot' { Read-ZixDkim $r.management_change }
        'management_apply' { Write-ZixDkim $r.management_change }
    }
    ConvertTo-Json -InputObject $result -Depth 12 -Compress
} catch {
    # Do not print provider exceptions, request/token, account or module paths.
    [Console]::Out.WriteLine('{"error":"exchange_operation_failed"}')
    exit 2
} finally {
    $r = $null
    if (Get-Command Disconnect-ExchangeOnline -ErrorAction SilentlyContinue) { Disconnect-ExchangeOnline -Confirm:$false -ErrorAction SilentlyContinue | Out-Null }
}
