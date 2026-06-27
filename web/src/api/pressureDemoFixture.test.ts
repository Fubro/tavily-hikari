import { describe, expect, it } from 'bun:test'

import { buildPressureDemoFixture } from './pressureDemoFixture'

describe('buildPressureDemoFixture', () => {
  it('keeps the mock pressure views internally consistent', () => {
    const snapshot = buildPressureDemoFixture(1_762_413_600, [])
    const latestCurrent = snapshot.server24h.current.at(-1)
    const latestPrevious = snapshot.server24h.previous.at(-1)
    const rowPressureSum = snapshot.currentUserDistribution.rows.reduce(
      (sum, row) => sum + row.pressure,
      0,
    )
    const rowFailureSum = snapshot.currentUserDistribution.rows.reduce(
      (sum, row) => sum + row.failureCount,
      0,
    )

    expect(latestCurrent).toBeDefined()
    expect(latestPrevious).toBeDefined()
    expect(snapshot.currentUserDistribution.rows.every((row) => row.pressure > 0)).toBe(true)
    expect(snapshot.currentUserDistribution.summary.currentPressure).toBe(
      latestCurrent?.pressure ?? 0,
    )
    expect(snapshot.currentUserDistribution.summary.vsYesterdayDelta).toBe(
      (latestCurrent?.pressure ?? 0) - (latestPrevious?.pressure ?? 0),
    )
    expect(rowPressureSum).toBe(snapshot.currentUserDistribution.summary.currentPressure)
    expect(rowFailureSum).toBe(latestCurrent?.failureCount ?? 0)
    expect(snapshot.currentUserDistribution.summary.peak).toBe(
      Math.max(...snapshot.currentUserDistribution.rows.map((row) => row.pressure), 0),
    )
    expect(snapshot.server7d.movingAverages).toHaveLength(2)
    expect(snapshot.server7d.movingAverages.every((series) => series.points.length === 168)).toBe(
      true,
    )
  })

  it('exposes repeated active-user pressure values so distribution charts can prove aggregation', () => {
    const snapshot = buildPressureDemoFixture(1_762_413_600, [])
    const frequencyByPressure = new Map<number, number>()

    for (const row of snapshot.currentUserDistribution.rows) {
      frequencyByPressure.set(row.pressure, (frequencyByPressure.get(row.pressure) ?? 0) + 1)
    }

    const duplicatePressures = [...frequencyByPressure.values()].filter((count) => count > 1)
    expect(duplicatePressures.length).toBeGreaterThan(0)
  })
})
