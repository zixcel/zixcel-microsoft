import { execFileSync } from 'node:child_process'
import { readFileSync, mkdtempSync, rmSync } from 'node:fs'
import { join } from 'node:path'
import { createHash } from 'node:crypto'
const manifest = JSON.parse(readFileSync('package.json'))
if (!/^@zixcel\/provider-(azure|google|cloudflare|microsoft|apple)$/.test(manifest.name) ||
    manifest.private === true || manifest.publishConfig?.access !== 'public' ||
    manifest.publishConfig?.registry !== 'https://registry.npmjs.org') throw new Error('Unexpected public package identity')
const directory = mkdtempSync(join(process.env.RUNNER_TEMP, 'provider-publication-'))
try {
 const packed = JSON.parse(execFileSync('npm', ['pack', '--ignore-scripts', '--json', '--pack-destination', directory]))
 const rows = Array.isArray(packed) ? packed : Object.values(packed)
 if (rows.length !== 1 || rows[0].name !== manifest.name || rows[0].version !== manifest.version ||
     !/^[a-z0-9.-]+\.tgz$/.test(rows[0].filename)) throw new Error('Candidate binding mismatch')
 const archive = join(directory, rows[0].filename), bytes = readFileSync(archive)
 const integrity = 'sha512-' + createHash('sha512').update(bytes).digest('base64')
 const response = await fetch(`https://registry.npmjs.org/${encodeURIComponent(manifest.name)}/${manifest.version}`,
   { signal: AbortSignal.timeout(15000), redirect: 'error' })
 if (response.ok) {
   const official = await response.json()
   if (official.name !== manifest.name || official.version !== manifest.version ||
       official.dist?.integrity !== integrity || !official.dist?.attestations?.url) throw new Error('Immutable official version mismatch')
   console.log(JSON.stringify({ name: manifest.name, version: manifest.version, state: 'existing-official-archive-matches', newOIDCPublication: false }))
 } else if (response.status === 404) {
   if (process.argv.includes('--verify-existing')) throw new Error('Existing official version required; publication disabled')
   execFileSync('npm', ['publish', archive, '--access', 'public', '--provenance', '--ignore-scripts'], { stdio: 'inherit' })
 } else throw new Error(`Registry version lookup failed: ${response.status}`)
 console.log(JSON.stringify({ name: manifest.name, version: manifest.version, sha256: createHash('sha256').update(bytes).digest('hex') }))
} finally { rmSync(directory, { recursive: true, force: true }) }
