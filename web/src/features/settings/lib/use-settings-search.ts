import { useMemo, useState } from 'react'
import { ACCOUNT_AREA, ADMIN_AREAS, useSettingsGroups, type ResolvedSettingsGroup, type SettingsArea } from '@/kernel/modules/settings'

const ACCOUNT_SCOPE: SettingsArea[] = [ACCOUNT_AREA]

function normalize(text: string): string {
  return text.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase()
}

export function useSettingsSearch(area: SettingsArea) {
  const [query, setQuery] = useState('')
  const scope = area.key === ACCOUNT_AREA.key ? ACCOUNT_SCOPE : ADMIN_AREAS
  const scoped = useSettingsGroups(scope)
  const groups = useMemo(() => scoped[scope.indexOf(area)] ?? [], [scoped, scope, area])

  const filteredGroups = useMemo<ResolvedSettingsGroup[]>(() => {
    const term = normalize(query.trim())
    if (!term) return groups
    return scoped
      .flat()
      .map((group) => ({
        ...group,
        items: group.items.filter((item) =>
          [group.label, item.label, item.description].some((text) => normalize(text).includes(term)),
        ),
      }))
      .filter((group) => group.items.length > 0)
  }, [groups, scoped, query])

  return { query, setQuery, groups, filteredGroups, isSearching: query.trim() !== '' }
}
