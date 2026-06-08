import { describe, expect, it } from 'bun:test'

import { resolveAdminUserActivityScope } from './userActivityScope'

describe('resolveAdminUserActivityScope', () => {
  it('defaults to active90d when the admin setting is enabled and the search query is empty', () => {
    expect(resolveAdminUserActivityScope('', true)).toBe('active90d')
    expect(resolveAdminUserActivityScope('   ', true)).toBe('active90d')
  })

  it('falls back to all users when the admin setting is disabled', () => {
    expect(resolveAdminUserActivityScope('', false)).toBe('all')
    expect(resolveAdminUserActivityScope('', undefined)).toBe('all')
    expect(resolveAdminUserActivityScope('', null)).toBe('all')
  })

  it('always searches across all users once a query is present', () => {
    expect(resolveAdminUserActivityScope('alice', true)).toBe('all')
    expect(resolveAdminUserActivityScope('  bob  ', true)).toBe('all')
    expect(resolveAdminUserActivityScope('charlie', false)).toBe('all')
  })
})
