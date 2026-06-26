export function buildDemoAnalysisPressureSnapshot(
  nowSeconds: (offset?: number) => number,
  filterDemoUsers: (url: URL) => Array<{
    userId: string
    displayName: string | null
    username: string | null
  }>,
) {
  const buildMovingAverage = (
    windowHours: number,
    points: Array<{
      bucketStart: number
      displayBucketStart: number
      pressure: number
    }>,
  ) => points.map((_point, index) => {
    const start = Math.max(0, index - windowHours + 1)
    const window = points.slice(start, index + 1)
    return {
      bucketStart: points[index]!.bucketStart,
      displayBucketStart: points[index]!.displayBucketStart,
      value: Math.round(window.reduce((sum, item) => sum + item.pressure, 0) / window.length),
    }
  })

  const base = nowSeconds()
  const current = Array.from({ length: 288 }, (_item, index) => {
    const displayBucketStart = base - (287 - index) * 300
    const pressure = Math.max(0, 18 + Math.round(Math.sin(index / 14) * 22) + ((index * 9) % 19))
    const failureCount = pressure > 0 ? Math.round(pressure * 0.12) : 0
    const successCount = Math.max(0, pressure - failureCount)
    return {
      bucketStart: displayBucketStart,
      displayBucketStart,
      pressure,
      successCount,
      failureCount,
    }
  })
  const previous = current.map((point, index) => {
    const pressure = Math.max(0, point.pressure - 6 + ((index * 5) % 11))
    const failureCount = pressure > 0 ? Math.round(pressure * 0.1) : 0
    const successCount = Math.max(0, pressure - failureCount)
    return {
      bucketStart: point.bucketStart - 86400,
      displayBucketStart: point.displayBucketStart,
      pressure,
      successCount,
      failureCount,
    }
  })
  const hourlyPoints = Array.from({ length: 168 }, (_item, index) => {
    const displayBucketStart = base - (167 - index) * 3600
    const pressure = Math.max(0, 22 + Math.round(Math.cos(index / 7) * 16) + ((index * 4) % 13))
    const failureCount = pressure > 0 ? Math.round(pressure * 0.09) : 0
    const successCount = Math.max(0, pressure - failureCount)
    return {
      bucketStart: displayBucketStart,
      displayBucketStart,
      pressure,
      successCount,
      failureCount,
    }
  })
  const samplePressures = [2, 4, 5, 7, 9, 11, 12, 16, 18, 19, 24, 31, 37, 49, 68]
  const rows = filterDemoUsers(new URL('https://demo.local/api/users')).slice(0, samplePressures.length).map((user, index) => {
    const pressure = samplePressures[index] ?? 0
    const failureCount = Math.max(1, Math.round(pressure * 0.08))
    const successCount = Math.max(0, pressure - failureCount)
    return {
      userId: user.userId,
      displayName: user.displayName,
      username: user.username,
      avatarUrl: null,
      pressure,
      successCount,
      failureCount,
    }
  })
  const currentPressure = rows.reduce((sum, row) => sum + row.pressure, 0)
  return {
    generatedAt: base,
    server24h: {
      windowMinutes: 60,
      bucketSeconds: 300,
      current,
      previous,
      currentPeak: current.reduce((best, point) => (!best || point.pressure > best.pressure ? point : best), null as typeof current[number] | null),
      previousPeak: previous.reduce((best, point) => (!best || point.pressure > best.pressure ? point : best), null as typeof previous[number] | null),
    },
    currentUserDistribution: {
      windowMinutes: 60,
      rows,
      summary: {
        activeUsers: rows.length,
        zeroPressureUsers: Math.max(0, filterDemoUsers(new URL('https://demo.local/api/users')).length - rows.length),
        median: 12,
        p90: 49,
        peak: rows.reduce((max, row) => Math.max(max, row.pressure), 0),
        currentPressure,
        vsYesterdayDelta: 14,
      },
    },
    server7d: {
      bucketSeconds: 3600,
      points: hourlyPoints,
      movingAverages: [
        { key: 'sma6h', windowHours: 6, points: buildMovingAverage(6, hourlyPoints) },
        { key: 'sma24h', windowHours: 24, points: buildMovingAverage(24, hourlyPoints) },
      ],
      peak: hourlyPoints.reduce((best, point) => (!best || point.pressure > best.pressure ? point : best), null as typeof hourlyPoints[number] | null),
    },
  }
}
