function Read-ZixConfiguration($packet) {
    $q=$packet.read;$data=$null;$extra=@{}
    try {
        $m=$null
        if($q.PSObject.Properties['mailbox']){$m=Get-ZixMailbox $q.mailbox;$identity="$($m.ExchangeGuid)"}
        switch($q.resource) {
            'organization' {
                $org=Get-OrganizationConfig -ErrorAction Stop;$transport=Get-TransportConfig -ErrorAction Stop
                $data=Select-ZixSettings $org @('AuditDisabled','DefaultAuthenticationPolicy')
                $data['SmtpClientAuthenticationDisabled']=[bool]$transport.SmtpClientAuthenticationDisabled
                $domains=Read-ZixConfigurationSection {Get-AcceptedDomain -ResultSize 201 -ErrorAction Stop} @('Name','DomainName','DomainType','Default')
                $data['accepted_domains']=$domains
                if(!$domains.complete){throw 'accepted_domains_unobserved'}
                $extra['authentication_scope']='Exchange settings only; Entra MFA/Conditional Access and Intune not inferred'
            }
            'policies' {return Read-ZixPolicies $q.kind}
            'mailbox' {$data=Select-ZixSettings $m @('ExternalDirectoryObjectId','PrimarySmtpAddress','MessageCopyForSentAsEnabled','MessageCopyForSendOnBehalfEnabled','HiddenFromAddressListsEnabled','RetentionPolicy','SharingPolicy','AddressBookPolicy')}
            'client_access' {
                $cas=Get-CASMailbox -Identity $identity -ErrorAction Stop
                $data=Select-ZixSettings $cas @('OWAEnabled','MAPIEnabled','ActiveSyncEnabled','PopEnabled','ImapEnabled','SmtpClientAuthenticationDisabled','OwaMailboxPolicy','ActiveSyncMailboxPolicy')
                $data['smtp_auth_null_means_inherit']=$true
            }
            'user_policy' {
                $u=Get-User -Identity "$($m.PrimarySmtpAddress)" -ErrorAction Stop
                if("$($u.ExternalDirectoryObjectId)".ToLowerInvariant() -ne $q.mailbox){throw 'user_policy_identity_mismatch'}
                $data=Select-ZixSettings $u @('AuthenticationPolicy')
                $data['null_means_organization_default']=$true
            }
            'inbox_rules' {
                $rules=@(Get-InboxRule -Mailbox $identity -IncludeHidden -ErrorAction Stop)
                if($rules.Count -gt 100){throw 'inbox_rule_limit'}
                $data=@{mailbox=$q.mailbox;primary="$($m.PrimarySmtpAddress)".ToLowerInvariant();rules=@($rules|ForEach-Object {Convert-ZixRule $_}|Sort-Object priority,identity)}
                $change=Get-ZixProperty $packet 'change'
                if($change -and $change.operation -eq 'set_inbox_rule_enabled' -and $change.enabled){
                    $selected=@($rules|Where-Object {"$($_.Identity)" -eq $change.identity})
                    if($selected.Count -ne 1){throw 'rule_identity_mismatch'}
                    $change=[pscustomobject]@{operation='create_inbox_rule';rule=(Convert-ZixRule $selected[0])}
                }
                $extra['routing_guard']=Read-ZixRoutingGuard $m $change
            }
            'folders' {return Read-ZixConfigurationSection {Get-EXOMailboxFolderStatistics -Identity $identity -ErrorAction Stop} @('FolderId','FolderPath','FolderType')}
            'folder_permissions' {return Read-ZixConfigurationSection {Get-EXOMailboxFolderPermission -Identity ($identity+':'+$q.folder) -ErrorAction Stop} @('User','AccessRights','SharingPermissionFlags')}
            'mobile_devices' {return Read-ZixConfigurationSection {Get-MobileDevice -Mailbox $identity -ResultSize 201 -ErrorAction Stop} @('Identity','DeviceId','DeviceType','DeviceModel','DeviceOS','ClientType','DeviceAccessState','DeviceAccessStateReason')}
            'auto_mapping' {
                $delegate=@(Get-EXORecipient -ExternalDirectoryObjectId $q.trustee -Properties ExternalDirectoryObjectId,PrimarySmtpAddress -ErrorAction Stop)
                if($delegate.Count -ne 1 -or "$($delegate[0].ExternalDirectoryObjectId)".ToLowerInvariant() -ne $q.trustee){throw 'delegate_mismatch'}
                $audit=Read-ZixMailboxAudit $q.mailbox;$selected=@();$other=@();$cache=@{}
                foreach($row in $audit.full_access_entries){if(Test-ZixTrustee $row.user $q.trustee "$($delegate[0].PrimarySmtpAddress)" $cache){$selected+=,$row}else{$other+=,$row}}
                foreach($row in $selected){$row.user="$($delegate[0].PrimarySmtpAddress)".ToLowerInvariant()}
                $data=@{mailbox=$q.mailbox;trustee=$q.trustee;trustee_smtp="$($delegate[0].PrimarySmtpAddress)".ToLowerInvariant();permissions=$selected;other_permissions=$other;send_as_entries=$audit.send_as_entries;email_addresses=$audit.email_addresses;auto_mapping='not_observable_by_permission_listing'}
            }
            'transport_rules' {return Read-ZixConfigurationSection {Get-TransportRule -ResultSize 201 -ErrorAction Stop} @('Identity','Name','State','Mode','Priority','Description','Conditions','Exceptions','Actions','StopRuleProcessing')}
            'connectors' {
                $inbound=Read-ZixConfigurationSection {Get-InboundConnector -ErrorAction Stop} @('Identity','Name','Enabled','ConnectorType','SenderDomains','SenderIPAddresses','RequireTls','TlsSenderCertificateName','RestrictDomainsToIPAddresses','RestrictDomainsToCertificate')
                $outbound=Read-ZixConfigurationSection {Get-OutboundConnector -ErrorAction Stop} @('Identity','Name','Enabled','ConnectorType','RecipientDomains','SmartHosts','UseMXRecord','TlsSettings','RouteAllMessagesViaOnPremises')
                return @{complete=($inbound.complete -and $outbound.complete);status='sections';data=@{inbound=$inbound;outbound=$outbound}}
            }
            'remote_domains' {return Read-ZixConfigurationSection {Get-RemoteDomain -ErrorAction Stop} @('Identity','Name','DomainName','AutoForwardEnabled','AllowedOOFType','DeliveryReportEnabled','NDREnabled')}
            'rbac' {
                $kind=Get-ZixProperty $q 'kind'
                if($kind -eq 'groups'){return Read-ZixConfigurationSection {Get-RoleGroup -ResultSize 201 -ErrorAction Stop} @('Identity','Name','Roles','Members')}
                if($kind -eq 'roles'){return Read-ZixConfigurationSection {Get-ManagementRole -ErrorAction Stop} @('Identity','Name','RoleType')}
                if($kind -eq 'assignments'){return Read-ZixConfigurationSection {Get-ManagementRoleAssignment -Role $q.identity -ErrorAction Stop} @('Identity','Name','Role','RoleAssigneeName','RoleAssigneeType','AssignmentMethod','Delegating','RecipientReadScope','RecipientWriteScope','CustomRecipientWriteScope')}
                $groups=Read-ZixConfigurationSection {Get-RoleGroup -ResultSize 201 -ErrorAction Stop} @('Identity','Name','Roles','Members')
                $assignments=Read-ZixConfigurationSection {Get-ManagementRoleAssignment -ErrorAction Stop} @('Identity','Name','Role','RoleAssigneeName','RoleAssigneeType','AssignmentMethod','Delegating','RecipientReadScope','RecipientWriteScope','CustomRecipientWriteScope')
                return @{complete=($groups.complete -and $assignments.complete);status='sections';data=@{groups=$groups;assignments=$assignments}}
            }
            default {throw 'unknown_configuration_resource'}
        }
        $result=@{complete=$true;status='observed';data=$data};foreach($key in $extra.Keys){$result[$key]=$extra[$key]};return $result
    }catch{return @{complete=$false;status='unobserved';reason='permission_service_or_shape_failure';data=$null}}
}
