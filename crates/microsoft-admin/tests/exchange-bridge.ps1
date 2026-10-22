$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/../src/exchange/bridge-read.ps1"
. "$PSScriptRoot/../src/exchange/bridge-write.ps1"
$script:writes = @()
function New-Mailbox { param($Shared,$Name,$DisplayName,$PrimarySmtpAddress,$ErrorAction) $script:writes += @{ kind='create'; display=$DisplayName; address=$PrimarySmtpAddress } }
function Set-Mailbox { param($Identity,$EmailAddresses,$ErrorAction) $script:writes += @{ kind='alias'; target=$Identity; delta=$EmailAddresses } }
$bad = "not code '; throw 'must not execute"
$c = [pscustomobject]@{operation='create_shared'; display_name=$bad; address='security@example.org'}
# Switch parameter behavior matches the official cmdlet; use a mock with a real switch.
Remove-Item Function:New-Mailbox
function New-Mailbox { param([switch]$Shared,$Name,$DisplayName,$PrimarySmtpAddress,$ErrorAction) if (!$Shared) { throw 'shared required' }; $script:writes += @{ kind='create'; display=$DisplayName; address=$PrimarySmtpAddress } }
$v=Write-ZixRecipient $c
if (!$v.acknowledged -or $script:writes.Count -ne 1 -or $script:writes[0].display -ne $bad) { throw 'literal mapping failure' }
Write-ZixRecipient ([pscustomobject]@{operation='add_alias'; mailbox='33333333-3333-3333-3333-333333333333'; address='help@example.org'}) | Out-Null
if ($script:writes[1].delta.Add -ne 'help@example.org') {throw 'alias replaced instead of added'}
$refused=$false
try { Write-ZixRecipient ([pscustomobject]@{operation='arbitrary'; command='whoami'}) } catch { $refused=$true }
if (!$refused -or $script:writes.Count -ne 2) {throw 'unknown operation accepted'}
$tokens=$null;$errors=$null
[System.Management.Automation.Language.Parser]::ParseFile("$PSScriptRoot/../src/exchange/bridge-main.ps1",[ref]$tokens,[ref]$errors) | Out-Null
if ($errors.Count -ne 0) {throw 'bridge syntax invalid'}
'Exchange command mapping and syntax tests passed; no module import, authentication or remote requests.'
