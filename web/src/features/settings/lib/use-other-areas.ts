import { useMemo } from 'react'
import { useAdminAreas, useAllowedSettingsAreas, type SettingsArea } from '@/kernel/modules/settings'

export function useOtherAreas(area: SettingsArea): SettingsArea[] {
  const allowed = useAllowedSettingsAreas(useAdminAreas())
  return useMemo(
    () => (area.scope === 'account' ? [] : allowed.filter((other) => other.key !== area.key)),
    [area, allowed],
  )
}
