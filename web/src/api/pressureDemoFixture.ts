import type {
  AnalysisCurrentUserPressureRow,
  AnalysisPressurePoint,
  AnalysisPressureSnapshot,
} from './adminAnalysis'

export interface PressureMockUserIdentity {
  userId: string
  displayName: string | null
  username: string | null
}

const USER_PRESSURE_WEIGHTS = [
  1,
  0.92,
  0.84,
  0.76,
  0.68,
  0.6,
  0.52,
  0.45,
  0.38,
  0.32,
  0.27,
  0.22,
  0.18,
  0.14,
]

const FALLBACK_USERS: PressureMockUserIdentity[] = [
  { userId: 'usr_olivia', displayName: 'Olivia Lin', username: 'olivia' },
  { userId: 'usr_owen', displayName: 'Owen Pei', username: 'owen' },
  { userId: 'usr_mika', displayName: 'Mika Du', username: 'mika' },
  { userId: 'usr_luna', displayName: 'Luna He', username: 'luna' },
  { userId: 'usr_alice', displayName: 'Alice Wang', username: 'alice' },
  { userId: 'usr_kevin', displayName: 'Kevin Shen', username: 'kevin' },
  { userId: 'usr_charlotte', displayName: 'Charlotte Gu', username: 'charlotte' },
  { userId: 'usr_jasper', displayName: 'Jasper Wu', username: 'jasper' },
  { userId: 'usr_bob', displayName: 'Bob Chen', username: 'bob' },
  { userId: 'usr_harper', displayName: 'Harper Xu', username: 'harper' },
  { userId: 'usr_charlie', displayName: 'Charlie Li', username: 'charlie' },
  { userId: 'usr_fiona', displayName: 'Fiona Qiu', username: 'fiona' },
  { userId: 'usr_daisy', displayName: 'Daisy Sun', username: 'daisy' },
  { userId: 'usr_ella', displayName: 'Ella Zhou', username: 'ella' },
  { userId: 'usr_iris', displayName: 'Iris Lin', username: 'iris' },
  { userId: 'usr_noa', displayName: 'Noa Jin', username: 'noa' },
  { userId: 'usr_cora', displayName: 'Cora Fang', username: 'cora' },
  { userId: 'usr_yuri', displayName: 'Yuri Gao', username: 'yuri' },
  { userId: 'usr_zoe', displayName: 'Zoe Wu', username: 'zoe' },
  { userId: 'usr_evan', displayName: 'Evan Luo', username: 'evan' },
  { userId: 'usr_hazel', displayName: 'Hazel Deng', username: 'hazel' },
  { userId: 'usr_ian', displayName: 'Ian Yu', username: 'ian' },
  { userId: 'usr_sophia', displayName: 'Sophia Sun', username: 'sophia' },
  { userId: 'usr_liam', displayName: 'Liam Qiu', username: 'liam' },
]

const WEEKDAY_FACTORS = [0.74, 0.96, 1.02, 1.06, 1.09, 1.01, 0.81]

function gaussian(value: number, center: number, width: number, amplitude: number): number {
  const normalized = (value - center) / width
  return Math.exp(-(normalized * normalized)) * amplitude
}

function roundPressure(value: number, minimum = 0): number {
  return Math.max(minimum, Math.round(value))
}

function localHour(timestampSeconds: number): number {
  const date = new Date(timestampSeconds * 1000)
  return date.getHours() + date.getMinutes() / 60
}

function weekdayIndex(timestampSeconds: number): number {
  return new Date(timestampSeconds * 1000).getDay()
}

function movingAverage(
  windowHours: number,
  points: Array<{
    bucketStart: number
    displayBucketStart: number
    pressure: number
  }>,
) {
  return points.map((_point, index) => {
    const start = Math.max(0, index - windowHours + 1)
    const window = points.slice(start, index + 1)
    return {
      bucketStart: points[index]!.bucketStart,
      displayBucketStart: points[index]!.displayBucketStart,
      value: Math.round(window.reduce((sum, item) => sum + item.pressure, 0) / window.length),
    }
  })
}

function buildDayProfile(hour: number): number {
  return (
    8 +
    gaussian(hour, 9.4, 2.4, 14) +
    gaussian(hour, 14.3, 3.1, 22) +
    gaussian(hour, 20.1, 2.2, 13) -
    gaussian(hour, 3.4, 2.5, 4)
  )
}

function build24hPressure(
  timestampSeconds: number,
  index: number,
  variant: 'current' | 'previous',
): number {
  const hour = localHour(timestampSeconds)
  const weekdayFactor = WEEKDAY_FACTORS[weekdayIndex(timestampSeconds)] ?? 1
  const base = buildDayProfile(hour) * weekdayFactor
  const microVariance = Math.sin(index / 8.5) * 1.3 + Math.cos(index / 3.8) * 0.8
  const lunchLift = gaussian(hour, 12.7, 1.1, variant === 'current' ? 2.4 : 1.2)
  const eveningCarry = gaussian(hour, 19.3, 1.5, variant === 'current' ? 2.1 : 0.7)
  const baselineShift = variant === 'current' ? 0.8 : -3.2
  return roundPressure(base + microVariance + lunchLift + eveningCarry + baselineShift, 4)
}

function build7dPressure(timestampSeconds: number, index: number): number {
  const hour = localHour(timestampSeconds)
  const weekdayFactor = WEEKDAY_FACTORS[weekdayIndex(timestampSeconds)] ?? 1
  const baseline =
    9 +
    gaussian(hour, 9.8, 2.8, 10) +
    gaussian(hour, 14.8, 3.2, 18) +
    gaussian(hour, 20.2, 2.4, 7) -
    gaussian(hour, 3.2, 2.3, 3)
  const slowTrend = Math.sin(index / 19) * 1.6 + Math.cos(index / 31) * 1.1
  const localVariance = Math.sin(index / 3.4) * 1.5 + Math.cos(index / 6.8) * 0.9
  const incidentLift =
    (weekdayIndex(timestampSeconds) === 3 ? gaussian(hour, 16.2, 1.4, 4) : 0) +
    (weekdayIndex(timestampSeconds) === 5 ? gaussian(hour, 11.5, 1.8, 2.2) : 0)
  return roundPressure(baseline * weekdayFactor + slowTrend + localVariance + incidentLift, 6)
}

function splitSuccessAndFailure(
  pressure: number,
  hour: number,
  baseFailureShare: number,
): Pick<AnalysisPressurePoint, 'successCount' | 'failureCount'> {
  const failureShare = baseFailureShare + gaussian(hour, 15.5, 3.2, 0.025) + gaussian(hour, 20.3, 2.0, 0.012)
  const failureCount = pressure <= 6 ? 0 : Math.max(1, Math.round(pressure * failureShare))
  const successCount = Math.max(0, pressure - failureCount)
  return { successCount, failureCount }
}

function allocateWeightedIntegers(
  total: number,
  weights: number[],
  options?: {
    minimumPerSlot?: number
    caps?: number[]
  },
): number[] {
  if (weights.length === 0 || total <= 0) return []
  const minimumPerSlot = Math.max(0, options?.minimumPerSlot ?? 0)
  const caps = options?.caps ?? weights.map(() => Number.POSITIVE_INFINITY)
  const allocations = weights.map(() => 0)
  let remaining = total

  if (minimumPerSlot > 0) {
    for (let index = 0; index < weights.length && remaining > 0; index += 1) {
      const grant = Math.min(minimumPerSlot, caps[index] ?? 0, remaining)
      allocations[index] = grant
      remaining -= grant
    }
  }

  while (remaining > 0) {
    let bestIndex = -1
    let bestScore = Number.NEGATIVE_INFINITY
    for (let index = 0; index < weights.length; index += 1) {
      const cap = caps[index] ?? Number.POSITIVE_INFINITY
      if (allocations[index] >= cap) continue
      const weight = Math.max(0.01, weights[index] ?? 0.01)
      const score = weight / (allocations[index] + 1)
      if (score > bestScore) {
        bestScore = score
        bestIndex = index
      }
    }
    if (bestIndex === -1) break
    allocations[bestIndex] += 1
    remaining -= 1
  }

  return allocations
}

function buildCurrentRows(
  userIdentities: PressureMockUserIdentity[],
  totalPressure: number,
  totalFailureCount: number,
): AnalysisCurrentUserPressureRow[] {
  const activeUserCount = Math.min(
    userIdentities.length,
    totalPressure,
    Math.max(5, Math.min(USER_PRESSURE_WEIGHTS.length, Math.round(totalPressure / 3.2))),
  )
  const weights = USER_PRESSURE_WEIGHTS.slice(0, activeUserCount)
  const pressures = allocateWeightedIntegers(totalPressure, weights, { minimumPerSlot: 1 })
  const failures = allocateWeightedIntegers(
    totalFailureCount,
    pressures.map((pressure, index) => pressure * (1 - index * 0.02)),
    { caps: pressures },
  )

  return userIdentities.slice(0, activeUserCount).map((identity, index) => {
    const pressure = pressures[index] ?? 0
    const failureCount = failures[index] ?? 0
    return {
      userId: identity.userId,
      displayName: identity.displayName,
      username: identity.username,
      avatarUrl: null,
      pressure,
      successCount: Math.max(0, pressure - failureCount),
      failureCount,
    }
  })
}

function median(values: number[]): number {
  if (values.length === 0) return 0
  const sorted = [...values].sort((left, right) => left - right)
  const middle = Math.floor(sorted.length / 2)
  if (sorted.length % 2 === 1) {
    return sorted[middle] ?? 0
  }
  const left = sorted[middle - 1] ?? 0
  const right = sorted[middle] ?? 0
  return Math.round((left + right) / 2)
}

function percentile(values: number[], ratio: number): number {
  if (values.length === 0) return 0
  const sorted = [...values].sort((left, right) => left - right)
  const rank = Math.min(sorted.length - 1, Math.max(0, Math.ceil(sorted.length * ratio) - 1))
  return sorted[rank] ?? 0
}

function peakPoint(points: AnalysisPressurePoint[]) {
  return points.reduce<AnalysisPressurePoint | null>(
    (best, point) => (!best || point.pressure > best.pressure ? point : best),
    null,
  )
}

function normalizeUserIdentities(
  inputUsers: PressureMockUserIdentity[],
  desiredCount: number,
): PressureMockUserIdentity[] {
  const seen = new Set<string>()
  const combined = [...inputUsers, ...FALLBACK_USERS]
  const result: PressureMockUserIdentity[] = []
  for (const user of combined) {
    const key = `${user.userId}:${user.username ?? ''}`
    if (seen.has(key)) continue
    seen.add(key)
    result.push(user)
    if (result.length >= desiredCount) break
  }
  return result
}

export function buildPressureDemoFixture(
  baseSeconds: number,
  inputUsers: PressureMockUserIdentity[],
): AnalysisPressureSnapshot {
  const userIdentities = normalizeUserIdentities(inputUsers, 24)
  const current = Array.from({ length: 288 }, (_item, index) => {
    const displayBucketStart = baseSeconds - (287 - index) * 300
    const pressure = build24hPressure(displayBucketStart, index, 'current')
    const hour = localHour(displayBucketStart)
    const { successCount, failureCount } = splitSuccessAndFailure(pressure, hour, 0.075)
    return {
      bucketStart: displayBucketStart,
      displayBucketStart,
      pressure,
      successCount,
      failureCount,
    }
  })
  const previous = Array.from({ length: 288 }, (_item, index) => {
    const displayBucketStart = baseSeconds - (287 - index) * 300
    const bucketStart = displayBucketStart - 86400
    const pressure = build24hPressure(bucketStart, index, 'previous')
    const hour = localHour(bucketStart)
    const { successCount, failureCount } = splitSuccessAndFailure(pressure, hour, 0.07)
    return {
      bucketStart,
      displayBucketStart,
      pressure,
      successCount,
      failureCount,
    }
  })
  const hourlyPoints = Array.from({ length: 168 }, (_item, index) => {
    const displayBucketStart = baseSeconds - (167 - index) * 3600
    const pressure = build7dPressure(displayBucketStart, index)
    const hour = localHour(displayBucketStart)
    const { successCount, failureCount } = splitSuccessAndFailure(pressure, hour, 0.068)
    return {
      bucketStart: displayBucketStart,
      displayBucketStart,
      pressure,
      successCount,
      failureCount,
    }
  })

  const latestCurrentPoint = current[current.length - 1]
  const latestPreviousPoint = previous[previous.length - 1]
  const rows = buildCurrentRows(
    userIdentities,
    latestCurrentPoint?.pressure ?? 0,
    latestCurrentPoint?.failureCount ?? 0,
  )
  const rowPressures = rows.map((row) => row.pressure)
  const currentPressure = rowPressures.reduce((sum, value) => sum + value, 0)
  const zeroPressureUsers = Math.max(0, userIdentities.length - rows.length)

  return {
    generatedAt: baseSeconds,
    server24h: {
      windowMinutes: 60,
      bucketSeconds: 300,
      current,
      previous,
      currentPeak: peakPoint(current),
      previousPeak: peakPoint(previous),
    },
    currentUserDistribution: {
      windowMinutes: 60,
      rows,
      summary: {
        activeUsers: rows.length,
        zeroPressureUsers,
        median: median(rowPressures),
        p90: percentile(rowPressures, 0.9),
        peak: Math.max(...rowPressures, 0),
        currentPressure,
        vsYesterdayDelta: currentPressure - (latestPreviousPoint?.pressure ?? 0),
      },
    },
    server7d: {
      bucketSeconds: 3600,
      points: hourlyPoints,
      movingAverages: [
        { key: 'sma6h', windowHours: 6, points: movingAverage(6, hourlyPoints) },
        { key: 'sma24h', windowHours: 24, points: movingAverage(24, hourlyPoints) },
      ],
      peak: peakPoint(hourlyPoints),
    },
  }
}
