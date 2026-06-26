import { buildPressureDemoFixture } from './pressureDemoFixture'

export function buildDemoAnalysisPressureSnapshot(
  nowSeconds: (offset?: number) => number,
  filterDemoUsers: (url: URL) => Array<{
    userId: string
    displayName: string | null
    username: string | null
  }>,
) {
  return buildPressureDemoFixture(
    nowSeconds(),
    filterDemoUsers(new URL('https://demo.local/api/users')),
  )
}
