export type AdminUserActivityScope = 'all' | 'active90d'

export function resolveAdminUserActivityScope(
  query: string,
  adminDefaultActiveUsersOnly: boolean | null | undefined,
): AdminUserActivityScope {
  if (query.trim().length > 0) return 'all'
  return adminDefaultActiveUsersOnly ? 'active90d' : 'all'
}
