import '../../test/happydom'

import { describe, expect, it } from 'bun:test'

import type { AnalysisCurrentUserPressureDistribution } from '../api'
import { buildActiveUserPressureDistribution } from './PressureAnalysisScreen'

describe('buildActiveUserPressureDistribution', () => {
  it('aggregates repeated pressure values into per-pressure user counts and drops zero-pressure users', () => {
    const distribution: AnalysisCurrentUserPressureDistribution = {
      windowMinutes: 60,
      rows: [
        {
          userId: 'usr-a',
          displayName: 'A',
          username: 'a',
          avatarUrl: null,
          pressure: 2,
          successCount: 2,
          failureCount: 0,
        },
        {
          userId: 'usr-b',
          displayName: 'B',
          username: 'b',
          avatarUrl: null,
          pressure: 2,
          successCount: 1,
          failureCount: 1,
        },
        {
          userId: 'usr-c',
          displayName: 'C',
          username: 'c',
          avatarUrl: null,
          pressure: 3,
          successCount: 3,
          failureCount: 0,
        },
        {
          userId: 'usr-d',
          displayName: 'D',
          username: 'd',
          avatarUrl: null,
          pressure: 4,
          successCount: 4,
          failureCount: 0,
        },
        {
          userId: 'usr-e',
          displayName: 'E',
          username: 'e',
          avatarUrl: null,
          pressure: 4,
          successCount: 3,
          failureCount: 1,
        },
        {
          userId: 'usr-f',
          displayName: 'F',
          username: 'f',
          avatarUrl: null,
          pressure: 4,
          successCount: 4,
          failureCount: 0,
        },
        {
          userId: 'usr-g',
          displayName: 'G',
          username: 'g',
          avatarUrl: null,
          pressure: 0,
          successCount: 0,
          failureCount: 0,
        },
      ],
      summary: {
        activeUsers: 6,
        zeroPressureUsers: 1,
        median: 3,
        p90: 4,
        peak: 4,
        currentPressure: 19,
        vsYesterdayDelta: 5,
      },
    }

    expect(buildActiveUserPressureDistribution(distribution)).toEqual([
      { pressure: 2, userCount: 2 },
      { pressure: 3, userCount: 1 },
      { pressure: 4, userCount: 3 },
    ])
  })
})
