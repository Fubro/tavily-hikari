import { describe, expect, it } from 'bun:test'

import { formatCompactSuccessRateValue, formatSuccessRateStackValue } from './shared'

describe('Admin screen shared format helpers', () => {
  it('keeps zero-sample success rates as no-data markers', () => {
    expect(formatSuccessRateStackValue(0, 0, 'en')).toEqual({
      primary: '—',
      secondary: 'S 0 / F 0',
    })

    expect(formatCompactSuccessRateValue(0, 0, 'en')).toBe('— S 0 / F 0')
    expect(formatCompactSuccessRateValue(0, 0, 'zh')).toBe('— 成 0 / 败 0')
  })
})
