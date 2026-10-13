import { discoveryOperations } from './discovery.mjs'
import { createLocalProviderControl, parameters, segment } from '@crowsi/provider-http-transport/local-control'
import type { CredentialSource, LocalAccount, LocalOperation } from '@crowsi/provider-http-transport/local-control'
const origin = 'https://graph.microsoft.com'
export const operations: readonly LocalOperation[] = Object.freeze([
  ...discoveryOperations,
  { id: 'account.read', effect: 'read', requiredPermissions: ['User.Read (delegated)'],
    build(input) { parameters(input, []); return { method: 'GET', url: `${origin}/v1.0/me?$select=id,displayName,userPrincipalName` } } },
  { id: 'mail.read', effect: 'read', requiredPermissions: ['Mail.ReadBasic (delegated)'],
    build(input) { parameters(input, []); return { method: 'GET', url: `${origin}/v1.0/me/messages?$top=20&$select=id,subject,isRead` } } },
  { id: 'mail.message-read', effect: 'read', requiredPermissions: ['Mail.ReadBasic (delegated)'],
    build(input) { const p = parameters(input, ['message']); return { method: 'GET',
      url: `${origin}/v1.0/me/messages/${segment(p.message)}?$select=id,isRead` } } },
  { id: 'mail.set-read', effect: 'write', requiredPermissions: ['Mail.ReadWrite (delegated)'],
    build(input) { const p = parameters(input, ['message', 'isRead']);
      if (typeof p.isRead !== 'boolean') throw new Error('provider/input/is-read')
      return { method: 'PATCH', url: `${origin}/v1.0/me/messages/${segment(p.message)}`, body: JSON.stringify({ isRead: p.isRead }) } } },
])
export function createControl(account: LocalAccount, credentials: CredentialSource, fetchImplementation?: typeof fetch, extensions: readonly LocalOperation[] = []) {
  if (account.provider !== 'microsoft-graph') throw new Error('provider/account/mismatch')
  segment(account.account)
  return createLocalProviderControl({ account, credentials, operations: [...operations, ...extensions], allowedOrigins: [origin], fetchImplementation })
}

export { cliCredentials } from './credentials.mjs'

export const serviceProfile = Object.freeze({
  provider: "microsoft-graph",
  accountScope: "Microsoft Entra tenant ID and delegated subject object ID.",
  credentialSource: "Owner-provided delegated credential; /me operations are not app-only APIs.",
  permissionCoverage: "HTTP acceptance or organization information is not consent/RBAC/effective API access inventory.",
  billingCoverage: "Billing coverage is not implied by Microsoft Graph organization or mail APIs.",
  permissionState: 'not-observed' as const,
  operations: Object.freeze(operations.map(({ id, effect, requiredPermissions }) =>
    Object.freeze({ id, effect, requiredPermissions: Object.freeze([...requiredPermissions]) }))),
})
