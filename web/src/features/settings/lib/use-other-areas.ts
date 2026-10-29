import { useMemo } from 'react'
import { ACCOUNT_AREA, ADMIN_AREAS, useAllowedSettingsAreas, type SettingsArea } from '@/kernel/modules/settings'

export function useOtherAreas(area: SettingsArea): SettingsArea[] {
  const allowed = useAllowedSettingsAreas(ADMIN_AREAS)
  return useMemo(
    () => (area.key === ACCOUNT_AREA.key ? [] : allowed.filter((other) => other.key !== area.key)),
    [area, allowed],
  )
}
