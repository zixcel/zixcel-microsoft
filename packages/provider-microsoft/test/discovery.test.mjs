import assert from 'node:assert/strict'
import test from 'node:test'
import { serviceProfile, operations } from '../dist/index.mjs'
test('public service profile exposes declarations without callbacks or claimed observed permissions', () => {
  assert.equal(serviceProfile.permissionState, 'not-observed')
  assert.equal(serviceProfile.operations.length, operations.length)
  assert.ok(Object.isFrozen(serviceProfile.operations))
  for (const operation of serviceProfile.operations) {
    assert.ok(Object.isFrozen(operation.requiredPermissions))
    assert.equal(operation.build, undefined)
  }
})
