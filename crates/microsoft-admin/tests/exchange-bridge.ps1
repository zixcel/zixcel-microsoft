$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../src/exchange/bridge-read.ps1"
. "$PSScriptRoot/../src/exchange/bridge-write.ps1"
$script:writes = @()
function Get-EXOMailbox { param($ExternalDirectoryObjectId,$Properties,$ErrorAction) [pscustomobject]@{ExternalDirectoryObjectId=$ExternalDirectoryObjectId; ExchangeGuid='55555555-5555-5555-5555-555555555555'} }
function Get-EXORecipient { param($ExternalDirectoryObjectId,$Properties,$ErrorAction) [pscustomobject]@{ExternalDirectoryObjectId=$ExternalDirectoryObjectId;PrimarySmtpAddress='delegate@example.org'} }
function New-Mailbox { param($Shared,$Name,$DisplayName,$PrimarySmtpAddress,$ErrorAction) $script:writes += @{ kind='create'; display=$DisplayName; address=$PrimarySmtpAddress } }
function Set-Mailbox { param($Identity,$EmailAddresses,$ErrorAction) $script:writes += @{ kind='alias'; target=$Identity; delta=$EmailAddresses } }
$bad = "not code '; throw 'must not execute"
$c = [pscustomobject]@{operation='create_shared'; display_name=$bad; address='security@example.org'}
# Switch parameter behavior matches the official cmdlet; use a mock with a real switch.
Remove-Item Function:New-Mailbox
function New-Mailbox { param([switch]$Shared,$Name,$Alias,$DisplayName,$PrimarySmtpAddress,$ErrorAction) if (!$Shared -or $Alias -ne $Name -or $Alias -notmatch '^shared-[a-f0-9]{20}$') { throw 'shared identity required' }; $script:writes += @{ kind='create'; display=$DisplayName; address=$PrimarySmtpAddress } }
$v=Write-ZixRecipient $c
if (!$v.acknowledged -or $script:writes.Count -ne 1 -or $script:writes[0].display -ne $bad) { throw 'literal mapping failure' }
Write-ZixRecipient ([pscustomobject]@{operation='add_alias'; mailbox='33333333-3333-3333-3333-333333333333'; address='help@example.org'}) | Out-Null
if ($script:writes[1].delta.Add -ne 'help@example.org') {throw 'alias replaced instead of added'}
$refused=$false
try { Write-ZixRecipient ([pscustomobject]@{operation='arbitrary'; command='whoami'}) } catch { $refused=$true }
if (!$refused -or $script:writes.Count -ne 2) {throw 'unknown operation accepted'}
function Add-MailboxPermission { param($Identity,$User,$AccessRights,$InheritanceType,$AutoMapping,$ErrorAction) $script:writes += @{kind='full'; target=$Identity; trustee=$User; rights=$AccessRights} }
function Remove-MailboxPermission { param($Identity,$User,$AccessRights,$InheritanceType,$Confirm,$ErrorAction) $script:writes += @{kind='remove-full'; target=$Identity; trustee=$User; rights=$AccessRights} }
function Add-RecipientPermission { param($Identity,$Trustee,$AccessRights,$Confirm,$ErrorAction) $script:writes += @{kind='send'; target=$Identity; trustee=$Trustee; rights=$AccessRights} }
function Remove-RecipientPermission { param($Identity,$Trustee,$AccessRights,$Confirm,$ErrorAction) $script:writes += @{kind='remove-send'; target=$Identity; trustee=$Trustee; rights=$AccessRights} }
foreach ($op in @('remove_alias','grant_full_access','revoke_full_access','grant_send_as','revoke_send_as')) {
    Write-ZixRecipient ([pscustomobject]@{operation=$op;mailbox='33333333-3333-3333-3333-333333333333';address='help@example.org';trustee='44444444-4444-4444-4444-444444444444'}) | Out-Null
}
if ($script:writes.Count -ne 7) {throw 'not every typed write was mapped exactly once'}
foreach ($w in ($script:writes | Select-Object -Skip 1)) { if ($w.target -ne '55555555-5555-5555-5555-555555555555') {throw 'Entra id was not resolved to Exchange id'} }
foreach ($w in ($script:writes | Select-Object -Skip 3)) { if ($w.trustee -ne 'delegate@example.org') {throw 'trustee identity mismatch'} }
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile("$PSScriptRoot/../src/exchange/bridge-main.ps1",[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {throw 'bridge syntax invalid'}
function Get-AcceptedDomain { param($ResultSize,$ErrorAction) [pscustomobject]@{DomainType='Authoritative';DomainName='example.org'} }
function Get-EXORecipient { param($ResultSize,$Properties,$ErrorAction)
    [pscustomobject]@{ExternalDirectoryObjectId='';PrimarySmtpAddress='discovery{55555555-5555-5555-5555-555555555555}@example.org';EmailAddresses=@('SMTP:discovery{55555555-5555-5555-5555-555555555555}@example.org');RecipientTypeDetails='DiscoveryMailbox';DisplayName='Synthetic discovery'}
}
$s=Read-ZixRecipients ([pscustomobject]@{scope=@{tenant='11111111-1111-1111-1111-111111111111';subject='22222222-2222-2222-2222-222222222222'};change=$null})
if (!$s.complete -or $s.recipients.Count -ne 1 -or $s.recipients[0].kind -ne 'DiscoveryMailbox' -or $s.recipients[0].id -ne ('exchange-system:'+$s.recipients[0].primary)) {throw 'system_recipient_not_retained'}
function Get-EXORecipient { param($ResultSize,$Properties,$ErrorAction)
    [pscustomobject]@{ExternalDirectoryObjectId='';PrimarySmtpAddress='user@example.org';EmailAddresses=@('SMTP:user@example.org');RecipientTypeDetails='UserMailbox';DisplayName='Synthetic user'}
}
$refused=$false
try { Read-ZixRecipients ([pscustomobject]@{scope=@{};change=$null}) } catch { $refused=$true }
if (!$refused) {throw 'missing_user_identity_accepted'}
'Exchange command mapping, system inventory and syntax tests passed; no module import, authentication or remote requests.'
. "$PSScriptRoot/../src/exchange/bridge-management.ps1"
function Get-AcceptedDomain { param($Identity,$ErrorAction) [pscustomobject]@{DomainType='Authoritative';DomainName=$Identity} }
function Get-DkimSigningConfig { param($Identity,$ErrorAction) [pscustomobject]@{Domain=$Identity;Enabled=$false;Selector1CNAME='Selector1-example-net._domainkey.Initial.onmicrosoft.com.';Selector2CNAME='selector2-example-net._domainkey.initial.onmicrosoft.com.'} }
function New-DkimSigningConfig { param($DomainName,$Enabled,$KeySize,$ErrorAction) if ($Enabled -ne $false -or $KeySize -ne 2048) {throw 'unsafe DKIM preparation'}; $script:writes += @{kind='dkim-new';domain=$DomainName} }
function Set-DkimSigningConfig { param($Identity,$Enabled,$ErrorAction) if ($Enabled -ne $true) {throw 'unexpected DKIM disable'}; $script:writes += @{kind='dkim-enable';domain=$Identity} }
$c=[pscustomobject]@{operation='prepare_dkim';domain='example.net'}
$s=Read-ZixDkim $c
if ($s.target.selector1 -ne 'selector1-example-net._domainkey.initial.onmicrosoft.com' -or $s.target.enabled -ne $false) {throw 'DKIM selectors not observed'}
Write-ZixDkim $c | Out-Null
Write-ZixDkim ([pscustomobject]@{operation='enable_dkim';domain='example.net'}) | Out-Null
if ($script:writes.Count -ne 9 -or $script:writes[-1].kind -ne 'dkim-enable') {throw 'DKIM mapping failure'}
function Get-DkimSigningConfig { param($Identity,$ErrorAction) throw 'permission denied' }
$refused=$false
try { Read-ZixDkim $c } catch { $refused=$true }
if (!$refused) {throw 'permission failure treated as absence'}
$refused=$false
try { Write-ZixDkim ([pscustomobject]@{operation='arbitrary'}) } catch { $refused=$true }
if (!$refused -or $script:writes.Count -ne 9) {throw 'unknown DKIM write accepted'}
'DKIM literal mapping, selector observation and permission failures passed without remote requests.'
