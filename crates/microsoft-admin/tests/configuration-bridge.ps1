$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
$root=Join-Path $PSScriptRoot '../src/exchange'
foreach($name in @('bridge-config-helpers.ps1','bridge-config-policies.ps1','bridge-config-rules.ps1','bridge-config-read.ps1','bridge-config-write.ps1')){. (Join-Path $root $name)}
function Assert($value,[string]$why){if(!$value){throw $why}}
$script:mailbox='33333333-3333-4333-8333-333333333333'
$script:delegate='44444444-4444-4444-8444-444444444444'
$script:calls=[System.Collections.Generic.List[string]]::new()
$script:rawRules=@();$script:failAdd=$false;$script:hasGrant=$true;$script:downstream=@{}
function Get-EXOMailbox {
    [CmdletBinding()]param($ExternalDirectoryObjectId,$Identity,$Properties)
    $id=if($ExternalDirectoryObjectId){$ExternalDirectoryObjectId}else{'55555555-5555-4555-8555-555555555555'}
    $address=if($Identity){$Identity}else{'support@example.org'}
    $forward=if($script:downstream.ContainsKey($address)){$script:downstream[$address]}else{''}
    [pscustomobject]@{ExternalDirectoryObjectId=$id;ExchangeGuid=$id;PrimarySmtpAddress=$address;EmailAddresses=@('SMTP:'+$address);MessageCopyForSentAsEnabled=$false;MessageCopyForSendOnBehalfEnabled=$false;HiddenFromAddressListsEnabled=$false;RetentionPolicy=$null;SharingPolicy='Default';AddressBookPolicy=$null;ForwardingSmtpAddress=$forward;ForwardingAddress=$null}
}
function Get-InboxRule {[CmdletBinding()]param($Mailbox,[switch]$IncludeHidden) if($Mailbox -eq $script:mailbox){$script:rawRules}}
function Get-AcceptedDomain {[CmdletBinding()]param($ResultSize) [pscustomobject]@{DomainType='Authoritative';DomainName='example.org'}}
function Get-EXORecipient {[CmdletBinding()]param($ExternalDirectoryObjectId,$Properties,$Identity) [pscustomobject]@{ExternalDirectoryObjectId=$script:delegate;PrimarySmtpAddress='delegate@example.org'}}
function New-InboxRule {
    [CmdletBinding(SupportsShouldProcess)]param($Mailbox,$Name,$Priority,$From,$SubjectContainsWords,$BodyContainsWords,$SubjectOrBodyContainsWords,$RedirectTo,$ForwardTo,[bool]$DeleteMessage,[bool]$StopProcessingRules,[switch]$AlwaysDeleteOutlookRulesBlob)
    $script:calls.Add('create')
    $script:rawRules+=,[pscustomobject]@{Identity='rule-1';Name=$Name;Priority=$Priority;Enabled=$true;From=$From;SubjectContainsWords=$SubjectContainsWords;BodyContainsWords=$BodyContainsWords;SubjectOrBodyContainsWords=$SubjectOrBodyContainsWords;RedirectTo=$RedirectTo;ForwardTo=$ForwardTo;DeleteMessage=$DeleteMessage;StopProcessingRules=$StopProcessingRules;IsHidden=$false;InError=$false;SupportedByTask=$true}
}
function Set-InboxRule {
    [CmdletBinding(SupportsShouldProcess)]param($Mailbox,$Identity,$Name,$Priority,$From,$SubjectContainsWords,$BodyContainsWords,$SubjectOrBodyContainsWords,$RedirectTo,$ForwardTo,[bool]$DeleteMessage,[bool]$StopProcessingRules,[switch]$AlwaysDeleteOutlookRulesBlob)
    $script:calls.Add('update')
    $r=$script:rawRules|Where-Object {$_.Identity -eq $Identity}
    foreach($key in @('Name','Priority','From','SubjectContainsWords','BodyContainsWords','SubjectOrBodyContainsWords','RedirectTo','ForwardTo','DeleteMessage','StopProcessingRules')){$r.$key=$PSBoundParameters[$key]}
}
function Remove-InboxRule {[CmdletBinding(SupportsShouldProcess)]param($Mailbox,$Identity,[switch]$AlwaysDeleteOutlookRulesBlob) $script:calls.Add('delete');$script:rawRules=@($script:rawRules|Where-Object {$_.Identity -ne $Identity})}
function Enable-InboxRule {[CmdletBinding(SupportsShouldProcess)]param($Mailbox,$Identity,[switch]$AlwaysDeleteOutlookRulesBlob) $script:calls.Add('enable');($script:rawRules|Where-Object {$_.Identity -eq $Identity}).Enabled=$true}
function Disable-InboxRule {[CmdletBinding(SupportsShouldProcess)]param($Mailbox,$Identity,[switch]$AlwaysDeleteOutlookRulesBlob) $script:calls.Add('disable');($script:rawRules|Where-Object {$_.Identity -eq $Identity}).Enabled=$false}
function Remove-MailboxPermission {[CmdletBinding(SupportsShouldProcess)]param($Identity,$User,$AccessRights,$InheritanceType) $script:calls.Add('remove-permission');$script:hasGrant=$false}
function Add-MailboxPermission {[CmdletBinding()]param($Identity,$User,$AccessRights,$InheritanceType,[bool]$AutoMapping) $script:calls.Add('add-permission:'+ $AutoMapping);if($script:failAdd){throw 'synthetic_failure'};$script:hasGrant=$true}
function Test-ZixTrustee($Identity,$Trustee,$Address,$Cache){$Identity -eq $Address}
function Read-ZixMailboxAudit($Mailbox){@{full_access_entries=@($(if($script:hasGrant){@{user='delegate@example.org';rights=@('FullAccess');deny=$false;inherited=$false}}));send_as_entries=@();email_addresses=@('SMTP:support@example.org')}}

function Get-User {
    [CmdletBinding()]param($Identity)
    Assert ($Identity -eq 'support@example.org') 'Get-User must use UUID-verified primary SMTP, not the unrelated ExchangeGuid'
    [pscustomobject]@{ExternalDirectoryObjectId=$script:mailbox;AuthenticationPolicy=$null}
}
function Set-User {
    [CmdletBinding()]param($Identity,$AuthenticationPolicy)
    Assert ($Identity -eq 'support@example.org' -and $AuthenticationPolicy -eq 'ExamplePolicy') 'Set-User must use the resolved target and exact settings'
}
$userPolicy=Read-ZixConfiguration ([pscustomobject]@{read=[pscustomobject]@{resource='user_policy';mailbox=$script:mailbox}})
Assert ($userPolicy.complete -and $userPolicy.data.null_means_organization_default) 'user policy assignment must be read with the verified target'
Write-ZixConfiguration ([pscustomobject]@{change=[pscustomobject]@{operation='configure';target=[pscustomobject]@{resource='user_policy';mailbox=$script:mailbox};values=[pscustomobject]@{AuthenticationPolicy='ExamplePolicy'}}})|Out-Null

$rule=[pscustomobject]@{name='zix:school';priority=0;from=@('school@example.org');subject_contains=@('$(throw "injected")');body_contains=@();subject_or_body_contains=@();delivery='redirect';recipients=@('child@example.org');keep_original=$false;stop_processing=$true;allow_external=$false}
$change=[pscustomobject]@{operation='create_inbox_rule';mailbox=$script:mailbox;rule=$rule;acknowledge_outlook_rules_blob_removal=$true}
$packet=[pscustomobject]@{read=[pscustomobject]@{resource='inbox_rules';mailbox=$script:mailbox};change=$change}
$before=Read-ZixConfiguration $packet
Assert $before.complete 'empty inbox rules must be observable'
Assert (!$before.routing_guard.known_cycle) 'internal empty destination must be acyclic'
$ack=Write-ZixConfiguration ([pscustomobject]@{change=$change});Assert $ack.acknowledged 'rule create acknowledgement'
$after=Read-ZixConfiguration $packet
Assert $after.complete 'created inbox rule must be observable'
Assert ($after.data.rules.Count -eq 1 -and $after.data.rules[0].manageable) 'new rule must roundtrip as managed'
Assert ($after.data.rules[0].body_contains.Count -eq 0) 'empty arrays must not become null members'
Assert ($after.data.rules[0].subject_contains[0] -eq '$(throw "injected")') 'keywords must remain literal data'
Assert (!$after.data.rules[0].keep_original) 'original disposition must roundtrip'
$change.operation='update_inbox_rule';$change|Add-Member identity 'rule-1';$rule.keep_original=$true
Write-ZixConfiguration ([pscustomobject]@{change=$change})|Out-Null
Assert (Read-ZixConfiguration $packet).data.rules[0].keep_original 'update must preserve intended original'
$toggle=[pscustomobject]@{operation='set_inbox_rule_enabled';mailbox=$script:mailbox;identity='rule-1';enabled=$false;acknowledge_outlook_rules_blob_removal=$true}
Write-ZixConfiguration ([pscustomobject]@{change=$toggle})|Out-Null
Assert (!$script:rawRules[0].Enabled) 'disable must use Disable-InboxRule'
$toggle.enabled=$true;Write-ZixConfiguration ([pscustomobject]@{change=$toggle})|Out-Null
Assert $script:rawRules[0].Enabled 'enable must use Enable-InboxRule'
$change.operation='delete_inbox_rule';Write-ZixConfiguration ([pscustomobject]@{change=$change})|Out-Null
Assert ($script:rawRules.Count -eq 0) 'delete must target one rule'
$change.operation='create_inbox_rule';$rule.recipients=@('support@example.org');Assert (Read-ZixConfiguration $packet).routing_guard.known_cycle 'self routing must be detected'
$rule.recipients=@('a@example.org');$script:downstream=@{'a@example.org'='b@example.org';'b@example.org'='a@example.org'}
Assert (Read-ZixConfiguration $packet).routing_guard.known_cycle 'downstream cycle must be detected'
$rule.recipients=@('outside@example.com');$script:downstream=@{}
Assert ((Read-ZixConfiguration $packet).routing_guard.external_destinations.Count -eq 1) 'external route must be explicit'
$script:calls.Clear();$script:hasGrant=$true;$script:failAdd=$true
$mapping=[pscustomobject]@{change=[pscustomobject]@{operation='set_auto_mapping';mailbox=$script:mailbox;trustee=$script:delegate;enabled=$true}}
try{Write-ZixConfiguration $mapping|Out-Null;throw 'expected partial failure'}catch{Assert ($_.Exception.Message -eq 'synthetic_failure') 'partial mutation failure must propagate'}
Assert (($script:calls -join ',') -eq 'remove-permission,add-permission:True') 'mapping must neither retry nor hide two-step failure'
Assert (!$script:hasGrant) 'partial state must stay visible'
$unavailable=Read-ZixConfigurationSection {throw 'synthetic-secret'} @('Name')
Assert (!$unavailable.complete -and $null -eq $unavailable.data) 'failed reads cannot mean empty configuration'
Assert ((ConvertTo-Json $unavailable -Compress) -notmatch 'synthetic-secret') 'provider exceptions must not escape'
'configuration bridge: all offline assertions passed; no provider connection or credential used'
