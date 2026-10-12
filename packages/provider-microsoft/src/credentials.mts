import { localCommandCredential } from '@crowsi/provider-http-transport/local-control'
import type { LocalAccount } from '@crowsi/provider-http-transport/local-control'
export function cliCredentials(account: LocalAccount, tenant: string) {
  const uuid = /^[a-f0-9]{8}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{4}-[a-f0-9]{12}$/i
  if (account.provider !== 'microsoft-graph' || !uuid.test(tenant) || !uuid.test(account.subject)
    || account.account.toLowerCase() !== tenant.toLowerCase()) {
    throw new Error('provider/credential/account-invalid')
  }
  return localCommandCredential('az', ['account', 'get-access-token', '--tenant', tenant,
    '--resource', 'https://graph.microsoft.com/', '--output', 'json'], text => {
      const result = JSON.parse(text)
      if (result.tenant !== tenant || typeof result.accessToken !== 'string'
        || !Number.isFinite(result.expires_on) || result.expires_on * 1000 <= Date.now() + 15000) {
        throw new Error('provider/credential/identity-or-expiry')
      }
      const payload = JSON.parse(Buffer.from(result.accessToken.split('.')[1], 'base64url').toString('utf8'))
      if (payload.oid !== account.subject || payload.tid !== tenant) throw new Error('provider/credential/subject')
      return result.accessToken
    }, account)
}
