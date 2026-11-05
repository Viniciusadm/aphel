import { useQuery } from '@tanstack/react-query'
import { useMemo } from 'react'
import type { ReferenceOptionsContext, ReferenceOptionsProvider, ReferenceOptionsResult } from '@/kernel/modules/define'
import {activitiesApi, activitiesKeys} from '@/sdk'

function useActivityTypeOptions({ includeInactive = false, enabled = true }: ReferenceOptionsContext): ReferenceOptionsResult {
  const types = useQuery({
    queryKey: activitiesKeys.types(includeInactive),
    queryFn: () => activitiesApi.listTypes(includeInactive),
    select: (page) => page.data,
    staleTime: 60_000,
    enabled,
  })
  return useMemo(
    () => ({
      options: (types.data ?? []).map((type) => ({ value: type.id, label: type.name })),
      isLoading: types.isPending,
    }),
    [types.data, types.isPending],
  )
}

export const ACTIVITY_TYPE_REFERENCE_OPTIONS: ReferenceOptionsProvider = {
  use: useActivityTypeOptions,
  queryKey: activitiesKeys.all,
}
