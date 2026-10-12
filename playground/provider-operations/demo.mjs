import assert from 'node:assert/strict'
import { createControl, operations } from '@zixcel/provider-microsoft'
const account = { provider: "microsoft-graph", subject: 'synthetic-owner', account: 'synthetic-account' }
const samples = {"account.read":{},"mail.read":{},"mail.set-read":{"message":"test-message","isRead":true},"mail.message-read":{"message":"test-message"}}
let state = {}, credentialUses = 0
const control = createControl(account, { acquire: async () => { credentialUses++; return 'synthetic-only-not-a-real-credential' } }, async (_url, init) => {
  if (init.method !== 'GET') state = JSON.parse(init.body)
  return new Response(JSON.stringify(state))
})
const write = operations.find(operation => operation.effect === 'write')
assert.ok(write)
const plan = control.prepare(write.id, samples[write.id])
const grant = control.approve(plan, Date.now() + 30000)
const changed = await control.execute(grant.id, plan)
const read = control.prepare('mail.message-read', samples['mail.message-read']), readGrant = control.approve(read, Date.now() + 30000)
const observed = await control.execute(readGrant.id, read)
assert.deepEqual(observed.data, changed.data)
const used = credentialUses
await assert.rejects(control.execute(grant.id, plan))
assert.equal(credentialUses, used)
console.log(JSON.stringify({ provider: account.provider, write: write.id, fakeReadBack: true, replayRefused: true, realAccount: false }))
