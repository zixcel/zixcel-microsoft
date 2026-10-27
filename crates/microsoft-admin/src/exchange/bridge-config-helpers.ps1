function Get-ZixProperty($object,[string]$name) {
    $p=$object.PSObject.Properties[$name]
    if ($null -eq $p) { return $null }
    return $p.Value
}
function Convert-ZixSetting($value) {
    if ($null -eq $value) { return $null }
    if ($value -is [bool] -or $value -is [int] -or $value -is [long] -or $value -is [uint32]) { return $value }
    if ($value -is [string]) { if ($value.Length -gt 2048) {throw 'configuration_value_limit'}; return $value }
    if ($value -is [System.Collections.IEnumerable]) { return ,@($value | ForEach-Object { Convert-ZixSetting $_ } | Sort-Object {"$_"}) }
    $s="$value"; if ($s.Length -gt 2048) {throw 'configuration_value_limit'}; return $s
}
function Select-ZixSettings($object,[string[]]$fields) {
    $row=@{}
    foreach($field in $fields) {
        if ($null -eq $object.PSObject.Properties[$field]) {throw 'configuration_property_missing'}
        if($field -eq 'Description'){$s="$($object.PSObject.Properties[$field].Value)";if($s.Length -gt 16384){throw 'configuration_description_limit'};$row[$field]=$s}
        else{$row[$field]=Convert-ZixSetting $object.PSObject.Properties[$field].Value}
        if ($field.EndsWith('Policy') -and !"$($row[$field])") {$row[$field]=$null}
    }
    return $row
}
function Read-ZixConfigurationSection([scriptblock]$reader,[string[]]$fields) {
    try {
        $rows=@(& $reader | Select-Object -First 201)
        if ($rows.Count -gt 200) {return @{complete=$false;status='partial';reason='item_limit';data=@()}}
        $selected=@($rows | ForEach-Object {Select-ZixSettings $_ $fields} | Sort-Object {ConvertTo-Json $_ -Compress -Depth 8})
        return @{complete=$true;status='observed';data=$selected}
    } catch {return @{complete=$false;status='unobserved';reason='permission_service_or_shape_failure';data=$null}}
}
function Get-ZixMailbox([string]$id) {
    $m=@(Get-EXOMailbox -ExternalDirectoryObjectId $id -Properties ExchangeGuid,ExternalDirectoryObjectId,PrimarySmtpAddress,EmailAddresses,MessageCopyForSentAsEnabled,MessageCopyForSendOnBehalfEnabled,HiddenFromAddressListsEnabled,RetentionPolicy,SharingPolicy,AddressBookPolicy -ErrorAction Stop)
    if ($m.Count -ne 1 -or "$($m[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $id) {throw 'configuration_mailbox_mismatch'}
    return $m[0]
}
function Get-ZixSmtp($recipient) {
    foreach($key in @('PrimarySmtpAddress','Address','SmtpAddress')) {
        $s="$(Get-ZixProperty $recipient $key)"; if ($s -match '^[^\s<>@]+@[^\s<>@]+$') {return $s.ToLowerInvariant()}
    }
    $s="$recipient"
    if ($s -match '^[^\s<>@]+@[^\s<>@]+$') {return $s.ToLowerInvariant()}
    if ($s -match '(?i)SMTP:([^\s\]<>@]+@[^\s\]<>@]+)') {return $Matches[1].ToLowerInvariant()}
    throw 'unresolved_rule_recipient'
}
function Get-ZixRouteEdges($m,$rules) {
    $edges=@()
    foreach($rule in $rules) {if (Get-ZixProperty $rule 'Enabled') {foreach($key in @('RedirectTo','ForwardTo','ForwardAsAttachmentTo')) {foreach($to in @(Get-ZixProperty $rule $key)) {if ("$to") {$edges+=Get-ZixSmtp $to}}}}}
    if ("$($m.ForwardingSmtpAddress)") {$edges+="$($m.ForwardingSmtpAddress)".Replace('smtp:','').ToLowerInvariant()}
    if ("$($m.ForwardingAddress)") {$to=Get-EXORecipient -Identity $m.ForwardingAddress -ErrorAction Stop;$edges+=Get-ZixSmtp $to}
    return ,@($edges | Sort-Object -Unique)
}
