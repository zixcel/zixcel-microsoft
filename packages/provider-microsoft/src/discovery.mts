import { parameters, segment } from '@crowsi/provider-http-transport/local-control'
import type { LocalOperation } from '@crowsi/provider-http-transport/local-control'
export const discoveryOperations: readonly LocalOperation[] = [
  { id: 'organization.read', effect: 'read', requiredPermissions: ['User.Read (delegated; supported Entra role)'],
    build(input, a) { parameters(input, []); return { method: 'GET',
      url: `https://graph.microsoft.com/v1.0/organization/${segment(a.account)}?$select=id,displayName,verifiedDomains` } } },
]
