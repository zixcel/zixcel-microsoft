function Read-ZixPolicies([string]$kind) {
    $identity=@('Identity','Name')
    switch($kind) {
        'authentication' {return Read-ZixConfigurationSection {Get-AuthenticationPolicy -ErrorAction Stop} ($identity+@('AllowBasicAuthPop','AllowBasicAuthImap','AllowBasicAuthSmtp'))}
        'mobile' {return Read-ZixConfigurationSection {Get-MobileDeviceMailboxPolicy -ErrorAction Stop} ($identity+@('IsDefault','PasswordEnabled','MinPasswordLength','DeviceEncryptionEnabled'))}
        'retention' {return Read-ZixConfigurationSection {Get-RetentionPolicy -ErrorAction Stop} ($identity+@('IsDefault','RetentionPolicyTagLinks'))}
        'sharing' {return Read-ZixConfigurationSection {Get-SharingPolicy -ErrorAction Stop} ($identity+@('Default','Enabled','Domains'))}
        'owa' {return Read-ZixConfigurationSection {Get-OwaMailboxPolicy -ErrorAction Stop} ($identity+@('IsDefault','DirectFileAccessOnPublicComputersEnabled','DirectFileAccessOnPrivateComputersEnabled'))}
        'address_book' {return Read-ZixConfigurationSection {Get-AddressBookPolicy -ErrorAction Stop} ($identity+@('GlobalAddressList','AddressLists','OfflineAddressBook','RoomList'))}
        'inbound_spam' {
            $policies=Read-ZixConfigurationSection {Get-HostedContentFilterPolicy -ErrorAction Stop} ($identity+@('IsDefault','SpamAction','HighConfidenceSpamAction','PhishSpamAction','HighConfidencePhishAction'))
            $bindings=Read-ZixConfigurationSection {Get-HostedContentFilterRule -ErrorAction Stop} ($identity+@('State','Priority','HostedContentFilterPolicy','SentTo','SentToMemberOf','RecipientDomainIs','ExceptIfSentTo','ExceptIfSentToMemberOf','ExceptIfRecipientDomainIs'))
        }
        'outbound_spam' {
            $policies=Read-ZixConfigurationSection {Get-HostedOutboundSpamFilterPolicy -ErrorAction Stop} ($identity+@('IsDefault','AutoForwardingMode','RecipientLimitExternalPerHour','RecipientLimitInternalPerHour','RecipientLimitPerDay'))
            $bindings=Read-ZixConfigurationSection {Get-HostedOutboundSpamFilterRule -ErrorAction Stop} ($identity+@('State','Priority','HostedOutboundSpamFilterPolicy','From','FromMemberOf','SenderDomainIs','ExceptIfFrom','ExceptIfFromMemberOf','ExceptIfSenderDomainIs'))
        }
        'anti_phish_advanced' {return Read-ZixConfigurationSection {Get-AntiPhishPolicy -Advanced -ErrorAction Stop} ($identity+@('IsDefault','EnableMailboxIntelligence','EnableMailboxIntelligenceProtection','EnableTargetedUserProtection','EnableTargetedDomainsProtection'))}
        'anti_phish' {
            $policies=Read-ZixConfigurationSection {Get-AntiPhishPolicy -ErrorAction Stop} ($identity+@('IsDefault','EnableSpoofIntelligence'))
            $bindings=Read-ZixConfigurationSection {Get-AntiPhishRule -ErrorAction Stop} ($identity+@('State','Priority','AntiPhishPolicy','SentTo','SentToMemberOf','RecipientDomainIs','ExceptIfSentTo','ExceptIfSentToMemberOf','ExceptIfRecipientDomainIs'))
        }
        default {throw 'unknown_policy_kind'}
    }
    $policyObservation=$policies.Clone();$policyObservation.Remove('data')
    return @{complete=($policies.complete -and $bindings.complete);status='sections';data=$policies.data;policy_observation=$policyObservation;bindings=$bindings}
}
