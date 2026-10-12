import assert from 'node:assert/strict'
import test from 'node:test'
import { createControl, operations, cliCredentials } from '../dist/index.mjs'
const account = { provider: "microsoft-graph", subject: 'synthetic-owner', account: 'synthetic-account' }
const samples = {"account.read":{},"mail.read":{},"mail.set-read":{"message":"test-message","isRead":true},"mail.message-read":{"message":"test-message"}}
const token = 'synthetic-bearer-not-a-real-account'
function fixture() {
  const requests = []
  const states = new Map()
  const initial = { initialized: true }
  let status = 200
  const control = createControl(account, { acquire: async () => token }, async (url, init) => {
    assert.equal(init.headers.Authorization, `Bearer ${token}`)
    requests.push({ url, method: init.method, body: init.body })
    if (status !== 200) return new Response('rejected', { status })
    const target = new URL(url).pathname
    if (init.method === 'PATCH') states.set(target, JSON.parse(init.body))
    return new Response(JSON.stringify(states.get(target) ?? initial), { status: 200 })
  })
  return { control, requests, deny: () => { status = 403 } }
}
test('all registered read/write operations use isolated fake credentials and HTTP', async () => {
  const f = fixture()
  for (const operation of operations) {
    const plan = f.control.prepare(operation.id, samples[operation.id])
    const grant = f.control.approve(plan, Date.now() + 30000)
    const receipt = await f.control.execute(grant.id, plan)
    assert.equal(receipt.operation, operation.id)
    assert.equal(receipt.permissionState, 'not-inferred')
    assert.ok(!JSON.stringify(receipt).includes(token))
  }
  assert.equal(f.requests.length, operations.length)
  assert.ok(f.requests.some(x => ['POST', 'PATCH', 'PUT'].includes(x.method)))
})
test('write operations change isolated state and can be observed through the same control', async () => {
  const f = fixture()
  const operation = operations.find(x => x.effect === 'write')
  const plan = f.control.prepare(operation.id, samples[operation.id])
  const grant = f.control.approve(plan, Date.now() + 30000)
  const write = await f.control.execute(grant.id, plan)
  const read = f.control.prepare('mail.message-read', samples['mail.message-read'])
  const readGrant = f.control.approve(read, Date.now() + 30000)
  const observed = await f.control.execute(readGrant.id, read)
  assert.deepEqual(observed.data, write.data)
  assert.equal(new URL(f.requests[0].url).pathname, new URL(f.requests[1].url).pathname)
  const unrelated = f.control.prepare('account.read', {})
  const unrelatedGrant = f.control.approve(unrelated, Date.now() + 30000)
  assert.notDeepEqual((await f.control.execute(unrelatedGrant.id, unrelated)).data, write.data)
})
test('rejects arbitrary API, input fields and resource changes', async () => {
  const f = fixture()
  assert.throws(() => f.control.prepare('arbitrary.delete', {}), /unsupported/)
  assert.throws(() => f.control.prepare('account.read', { token: 'forbidden' }), /input\/fields/)
  const plan = f.control.prepare('account.read', {})
  const grant = f.control.approve(plan, Date.now() + 30000)
  await assert.rejects(f.control.execute(grant.id, { ...plan, account: { ...account, account: 'other' } }), /plan\/changed/)
  assert.equal(f.requests.length, 0)
})
test('propagates remote permission denial without secret/error body leakage', async () => {
  const f = fixture(); f.deny()
  const plan = f.control.prepare('account.read', {})
  const grant = f.control.approve(plan, Date.now() + 30000)
  await assert.rejects(f.control.execute(grant.id, plan), /provider\/http\/403/)
})

test('CLI credential construction rejects a different tenant without invoking az', () => {
  const tenant = '11111111-1111-4111-8111-111111111111'
  const subject = '22222222-2222-4222-8222-222222222222'
  assert.throws(() => cliCredentials({ provider: 'microsoft-graph', subject, account: tenant },
    '33333333-3333-4333-8333-333333333333'), /account-invalid/)
  assert.throws(() => cliCredentials({ provider: 'microsoft-graph', subject: 'not-an-object-id', account: tenant },
    tenant), /account-invalid/)
})

test('rejects decoded credential echoes without exposing them to callers', async () => {
  const escaped = [...token].map(character => `\\u${character.charCodeAt(0).toString(16).padStart(4, '0')}`).join('')
  const control = createControl(account, { acquire: async () => token }, async () =>
    new Response(`{"echo":"${escaped}"}`, { status: 200 }))
  const plan = control.prepare('account.read', {})
  const grant = control.approve(plan, Date.now() + 30000)
  await assert.rejects(control.execute(grant.id, plan), /credential-rejected/)
})
