import { describe, expect, it } from 'bun:test'

import { countAdminJobGroups, jobMatchesGroup } from './jobFilters'
import type { JobLogView } from '../api'

describe('admin job filters', () => {
  it('groups LinuxDo tag refresh jobs with LinuxDo maintenance', () => {
    expect(jobMatchesGroup('linuxdo_user_tag_binding_refresh', 'linuxdo')).toBe(true)

    const jobs: JobLogView[] = [
      {
        id: 1,
        job_type: 'linuxdo_user_tag_binding_refresh',
        trigger_source: 'manual',
        key_id: null,
        key_group: null,
        status: 'success',
        attempt: 1,
        message: null,
        started_at: 1,
        finished_at: 2,
      },
    ]

    expect(countAdminJobGroups(jobs).linuxdo).toBe(1)
  })
})
