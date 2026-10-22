import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { mkdtempSync, readFileSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join, resolve } from 'node:path'
import { spawnSync } from 'node:child_process'
const [dispatcherArg, binaryArg, manifestArg] = process.argv.slice(2)
assert(dispatcherArg && binaryArg && manifestArg)
const dispatcher = resolve(dispatcherArg), binary = resolve(binaryArg), manifest = resolve(manifestArg)
const root = mkdtempSync(join(tmpdir(), 'microsoft-dispatcher-'))
const config = join(root, 'config'); mkdirSync(config, { mode: 0o700 })
const environment = { PATH: '/usr/bin:/bin', HOME: root, XDG_CONFIG_HOME: config }
function run(args: readonly string[]) {
  const r = spawnSync(dispatcher, args, { env: environment, encoding: 'utf8', timeout: 8_000, maxBuffer: 131_072 })
  assert.equal(r.error, undefined, 'bounded dispatcher failed')
  assert.equal(r.signal, null)
  return r
}
const digest = (path: string) => createHash('sha256').update(readFileSync(path)).digest('hex')
assert.equal(run(['extensions', 'add', '--manifest', manifest, '--manifest-sha256', digest(manifest), '--executable', binary, '--executable-sha256', digest(binary)]).status, 0)
const scope = ['--tenant', '11111111-1111-1111-1111-111111111111', '--subject', '22222222-2222-2222-2222-222222222222']
const cases: readonly [readonly string[], string | null][] = [
  [['microsoft', 'admin', 'doctor'], null],
  [['microsoft', 'domains', 'list', ...scope], 'credential_missing'],
  [['microsoft', 'licenses', 'list', ...scope], 'credential_missing'],
  [['microsoft', 'users', 'get', '--user', '33333333-3333-3333-3333-333333333333'], 'credential_missing'],
  [['microsoft', 'domains', 'dns-records', ...scope, '--domain', 'https://example.org'], 'invalid_domain'],
]
for (const [args, expected] of cases) {
  const r = run(args)
  const value = JSON.parse(r.stdout || r.stderr) as { error?: string; result?: Record<string, unknown> }
  if (expected) assert.equal(value.error, expected)
  else {
    assert.equal(r.status, 0)
    assert.equal(value.result?.credentials_read, false)
    assert.equal(value.result?.remote_requests, false)
  }
}
const r = run(['microsoft', 'exchange', 'apply', ...scope, '--plan-file', join(root, 'absent-plan.json'), '--approval-sha256', 'a'.repeat(64), '--journal-directory', join(root, 'absent-journal')])
assert.notEqual(r.status, 0)
assert.match(r.stdout + r.stderr, /interactive_authorization_required/)
const result = { passed: cases.length + 1, credentialInput: false, liveRequests: false, remoteWrites: false, isolatedRegistry: true }
writeFileSync(join(root, 'verification.json'), JSON.stringify(result), { mode: 0o600 })
process.stdout.write(`${JSON.stringify(result)}\n`)
