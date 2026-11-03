import { useCallback, useMemo } from 'react'
import { useSearchParams } from 'react-router'
import { isDayKey } from '@/sdk/calendar/dates'
import { parseViewMode, type CalendarViewMode } from './lib/range'

export interface CalendarUrlState {
  mode: CalendarViewMode
  date: string | null
  ownerId: string
  teamId: string
  typeId: string
  sourceId: string
}

const PARAMS = { mode: 'view', date: 'date', owner: 'owner', team: 'team', type: 'type', source: 'source' } as const

export function useCalendarUrlState() {
  const [params, setParams] = useSearchParams()
  const serialized = params.toString()
  const state = useMemo<CalendarUrlState>(() => {
    const current = new URLSearchParams(serialized)
    const date = current.get(PARAMS.date)
    return {
      mode: parseViewMode(current.get(PARAMS.mode)),
      date: isDayKey(date) ? date : null,
      ownerId: current.get(PARAMS.owner) ?? '',
      teamId: current.get(PARAMS.team) ?? '',
      typeId: current.get(PARAMS.type) ?? '',
      sourceId: current.get(PARAMS.source) ?? '',
    }
  }, [serialized])

  const update = useCallback(
    (patch: Partial<CalendarUrlState>) =>
      setParams(
        (current) => {
          const next = new URLSearchParams(current)
          const set = (key: string, value: string | null | undefined) => {
            if (value) next.set(key, value)
            else next.delete(key)
          }
          if (patch.mode !== undefined) set(PARAMS.mode, patch.mode === 'month' ? null : patch.mode)
          if (patch.date !== undefined) set(PARAMS.date, patch.date)
          if (patch.ownerId !== undefined) set(PARAMS.owner, patch.ownerId)
          if (patch.teamId !== undefined) set(PARAMS.team, patch.teamId)
          if (patch.sourceId !== undefined) set(PARAMS.source, patch.sourceId)
          if (patch.typeId !== undefined) set(PARAMS.type, patch.typeId)
          return next
        },
        { replace: true },
      ),
    [setParams],
  )

  return [state, update] as const
}
