function Write-ZixConfiguration($packet) {
    $c=$packet.change
    switch($c.operation) {
        'configure' {
            $values=@{};foreach($p in $c.values.PSObject.Properties){$values[$p.Name]=$p.Value}
            $q=$c.target
            switch($q.resource) {
                'mailbox' {$m=Get-ZixMailbox $q.mailbox;Set-Mailbox -Identity "$($m.ExchangeGuid)" @values -ErrorAction Stop}
                'client_access' {$m=Get-ZixMailbox $q.mailbox;Set-CASMailbox -Identity "$($m.ExchangeGuid)" @values -ErrorAction Stop}
                'user_policy' {$m=Get-ZixMailbox $q.mailbox;Set-User -Identity "$($m.ExchangeGuid)" @values -ErrorAction Stop}
                'organization' {Set-TransportConfig @values -ErrorAction Stop}
                'remote_domains' {Set-RemoteDomain -Identity $c.identity @values -ErrorAction Stop}
                'policies' {switch($q.kind) {
                    'mobile' {Set-MobileDeviceMailboxPolicy -Identity $c.identity @values -ErrorAction Stop}
                    'authentication' {Set-AuthenticationPolicy -Identity $c.identity @values -ErrorAction Stop}
                    'outbound_spam' {Set-HostedOutboundSpamFilterPolicy -Identity $c.identity @values -ErrorAction Stop}
                    default {throw 'unsupported_policy_write'}
                }}
                default {throw 'unsupported_configuration_write'}
            }
        }
        'set_auto_mapping' {
            $m=Get-ZixMailbox $c.mailbox
            $d=@(Get-EXORecipient -ExternalDirectoryObjectId $c.trustee -Properties ExternalDirectoryObjectId,PrimarySmtpAddress -ErrorAction Stop)
            if($d.Count -ne 1 -or "$($d[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $c.trustee){throw 'trustee_mismatch'}
            # Two explicit, reviewed writes. A failure between them stays unknown;
            # never repeat this sequence automatically or claim atomicity.
            $audit=Read-ZixConfiguration ([pscustomobject]@{read=[pscustomobject]@{resource='auto_mapping';mailbox=$c.mailbox;trustee=$c.trustee}})
            if(!$audit.complete){throw 'permission_preflight_failed'}
            if($audit.data.permissions.Count -gt 0){Remove-MailboxPermission -Identity "$($m.ExchangeGuid)" -User "$($d[0].PrimarySmtpAddress)" -AccessRights FullAccess -InheritanceType All -Confirm:$false -ErrorAction Stop}
            Add-MailboxPermission -Identity "$($m.ExchangeGuid)" -User "$($d[0].PrimarySmtpAddress)" -AccessRights FullAccess -InheritanceType All -AutoMapping:([bool]$c.enabled) -ErrorAction Stop | Out-Null
        }
        {$_ -in @('create_inbox_rule','update_inbox_rule','delete_inbox_rule','set_inbox_rule_enabled')} {Write-ZixConfigurationRule $c}
        default {throw 'unknown_configuration_change'}
    }
    return @{acknowledged=$true}
}
function Write-ZixConfigurationRule($c) {
    if(!$c.acknowledge_outlook_rules_blob_removal){throw 'outlook_rules_blob_not_acknowledged'}
    $m=Get-ZixMailbox $c.mailbox;$mailbox="$($m.ExchangeGuid)"
    if($c.operation -eq 'set_inbox_rule_enabled') {
        if($c.enabled){Enable-InboxRule -Mailbox $mailbox -Identity $c.identity -AlwaysDeleteOutlookRulesBlob -Confirm:$false -ErrorAction Stop}
        else{Disable-InboxRule -Mailbox $mailbox -Identity $c.identity -AlwaysDeleteOutlookRulesBlob -Confirm:$false -ErrorAction Stop}
        return
    }
    if($c.operation -eq 'delete_inbox_rule') {
        Remove-InboxRule -Mailbox $mailbox -Identity $c.identity -AlwaysDeleteOutlookRulesBlob -Confirm:$false -ErrorAction Stop
        return
    }
    $r=$c.rule
    $settings=@{Name=$r.name;Priority=[int]$r.priority;From=@($r.from);SubjectContainsWords=@($r.subject_contains);BodyContainsWords=@($r.body_contains);SubjectOrBodyContainsWords=@($r.subject_or_body_contains);RedirectTo=@();ForwardTo=@();DeleteMessage=(!$r.keep_original);StopProcessingRules=[bool]$r.stop_processing}
    # Empty collections clear precisely these supported conditions/actions.
    foreach($key in @('From','SubjectContainsWords','BodyContainsWords','SubjectOrBodyContainsWords','RedirectTo','ForwardTo')){if(!$settings[$key].Count){$settings[$key]=$null}}
    if($r.delivery -eq 'redirect'){$settings.RedirectTo=@($r.recipients)}else{$settings.ForwardTo=@($r.recipients)}
    if($c.operation -eq 'create_inbox_rule'){
        New-InboxRule -Mailbox $mailbox @settings -AlwaysDeleteOutlookRulesBlob -Confirm:$false -ErrorAction Stop | Out-Null
    }else{
        Set-InboxRule -Mailbox $mailbox -Identity $c.identity @settings -AlwaysDeleteOutlookRulesBlob -Confirm:$false -ErrorAction Stop
    }
}
