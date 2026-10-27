function Test-ZixRuleValue($value) {
    if ($null -eq $value -or $value -eq $false) {return $false}
    foreach($v in @($value)) {if ("$v" -and "$v" -notin @('None','Any','0','False')) {return $true}}
    return $false
}
function Convert-ZixRule($r) {
    $unsupported=@()
    $allowed=@('From','SubjectContainsWords','BodyContainsWords','SubjectOrBodyContainsWords','RedirectTo','ForwardTo','DeleteMessage','StopProcessingRules')
    $metadata=@('Name','Identity','RuleIdentity','Priority','Enabled','Description','SupportedByTask','IsValid','InError','ErrorType','MailboxOwnerId','RuleProvider','RuleVersion','ObjectState','IsReadOnly','IsHidden','IsEnabled','RunspaceId','PSComputerName','PSShowComputerName','Guid','DistinguishedName','OrganizationId','ExchangeVersion')
    foreach($p in $r.PSObject.Properties) {
        if ($p.Name -in $allowed -or $p.Name -in $metadata) {continue}
        if (Test-ZixRuleValue $p.Value) {$unsupported+=$p.Name}
    }
    $redirect=@();$forward=@();$from=@();$resolution=$true
    try {foreach($v in @(Get-ZixProperty $r 'From')) {if ("$v") {$from+=Get-ZixSmtp $v}};foreach($v in @(Get-ZixProperty $r 'RedirectTo')) {if ("$v") {$redirect+=Get-ZixSmtp $v}};foreach($v in @(Get-ZixProperty $r 'ForwardTo')) {if ("$v") {$forward+=Get-ZixSmtp $v}}} catch {$resolution=$false}
    $delivery=if($redirect.Count){'redirect'}elseif($forward.Count){'forward'}else{'none'}
    $manageable="$($r.Name)".StartsWith('zix:') -and !$unsupported.Count -and $resolution -and !($redirect.Count -and $forward.Count) -and !(Get-ZixProperty $r 'IsHidden') -and !(Get-ZixProperty $r 'InError')
    $subject=@(@(Get-ZixProperty $r 'SubjectContainsWords')|Where-Object {$null -ne $_ -and "$_"}|ForEach-Object {"$_"})
    $body=@(@(Get-ZixProperty $r 'BodyContainsWords')|Where-Object {$null -ne $_ -and "$_"}|ForEach-Object {"$_"})
    $either=@(@(Get-ZixProperty $r 'SubjectOrBodyContainsWords')|Where-Object {$null -ne $_ -and "$_"}|ForEach-Object {"$_"})
    @{identity="$($r.Identity)";name="$($r.Name)";priority=[int]$r.Priority;enabled=[bool]$r.Enabled;from=@($from|Sort-Object);subject_contains=$subject;body_contains=$body;subject_or_body_contains=$either;delivery=$delivery;recipients=@(($redirect+$forward)|Sort-Object);keep_original=![bool](Get-ZixProperty $r 'DeleteMessage');stop_processing=[bool](Get-ZixProperty $r 'StopProcessingRules');manageable=[bool]$manageable;unsupported_properties=@($unsupported|Sort-Object)}
}
function Read-ZixRoutingGuard($mailbox,$change) {
    $result=@{complete=$false;evaluated=$false;known_cycle=$null;external_destinations=@();scope='mailbox_forwarding_and_inbox_rules_only';nodes=@()}
    if (!$change -or $change.operation -notin @('create_inbox_rule','update_inbox_rule')) {return $result}
    $result.complete=$true;$result.evaluated=$true;$result.known_cycle=$false
    $root=@($mailbox.EmailAddresses|ForEach-Object {"$_".Replace('SMTP:','').Replace('smtp:','').ToLowerInvariant()})
    $domains=@(Get-AcceptedDomain -ResultSize 201 -ErrorAction Stop)
    if ($domains.Count -gt 200) {throw 'routing_domain_limit'}
    $internal=@($domains|Where-Object {$_.DomainType -eq 'Authoritative'}|ForEach-Object {"$($_.DomainName)".ToLowerInvariant()})
    $queue=[System.Collections.Generic.Queue[string]]::new();foreach($to in $change.rule.recipients) {$queue.Enqueue($to)}
    $seen=@{};$edges=@{}
    while($queue.Count) {
        $address=$queue.Dequeue().ToLowerInvariant()
        if ($root -contains $address) {$result.known_cycle=$true;continue}
        if ($seen.ContainsKey($address)) {continue};$seen[$address]=$true
        if ($seen.Count -gt 16) {throw 'routing_graph_limit'}
        if ($address.Split('@')[-1] -notin $internal) {$result.external_destinations+=,$address;continue}
        $recipient=@(Get-EXOMailbox -Identity $address -Properties ExchangeGuid,ExternalDirectoryObjectId,EmailAddresses,ForwardingAddress,ForwardingSmtpAddress -ErrorAction Stop)
        if ($recipient.Count -ne 1) {throw 'routing_destination_ambiguous'}
        $node=$recipient[0];$alias=@($node.EmailAddresses|ForEach-Object {"$_".Replace('SMTP:','').Replace('smtp:','').ToLowerInvariant()})
        if (@($alias|Where-Object {$_ -in $root}).Count) {$result.known_cycle=$true;continue}
        $id="$($node.ExternalDirectoryObjectId)";if(!$id){throw 'routing_destination_identity_missing'}
        $rules=@(Get-InboxRule -Mailbox "$($node.ExchangeGuid)" -IncludeHidden -ErrorAction Stop)
        if($rules.Count -gt 100){throw 'routing_rule_limit'}
        $next=@(Get-ZixRouteEdges $node $rules);$edges[$address]=$next
        $result.nodes+=,@{address=$address;mailbox=$id;aliases=$alias;destinations=$next}
        foreach($to in $next){$queue.Enqueue($to)}
    }
    # A bounded DFS also rejects cycles wholly inside the downstream subgraph.
    function Test-ZixCycle([string]$node,$edges,$visiting,$done) {
        if($visiting.ContainsKey($node)){return $true};if($done.ContainsKey($node)){return $false}
        $visiting[$node]=$true
        if($edges.ContainsKey($node)){foreach($to in $edges[$node]){if(Test-ZixCycle $to $edges $visiting $done){return $true}}}
        $visiting.Remove($node);$done[$node]=$true;return $false
    }
    $visiting=@{};$done=@{};foreach($node in @($edges.Keys)){if(Test-ZixCycle $node $edges $visiting $done){$result.known_cycle=$true}}
    $result.external_destinations=@($result.external_destinations|Sort-Object -Unique)
    return $result
}
