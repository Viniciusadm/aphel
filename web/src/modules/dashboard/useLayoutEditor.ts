import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { dashboardApi, dashboardKeys } from './api'
import type { DashboardLayout, SaveLayoutRequest } from './types'

export function useLayoutEditor() {
  const { t } = useTranslation('dashboard')
  const queryClient = useQueryClient()
  const key = dashboardKeys.layout()

  const save = useMutation({
    mutationFn: (layout: SaveLayoutRequest) => dashboardApi.saveLayout(layout),
    onMutate: async (layout) => {
      await queryClient.cancelQueries({ queryKey: key })
      const previous = queryClient.getQueryData<DashboardLayout>(key)
      queryClient.setQueryData<DashboardLayout>(key, {
        ...layout,
        compare_to_previous: layout.compare_to_previous ?? false,
        is_default: false,
      })
      return { previous }
    },
    onError: (error, _layout, context) => {
      if (context?.previous) queryClient.setQueryData(key, context.previous)
      notifyError(error, { title: t('states.saveError') })
    },
    onSuccess: (layout) => queryClient.setQueryData(key, layout),
  })

  const reset = useMutation({
    mutationFn: dashboardApi.resetLayout,
    onSuccess: (layout) => queryClient.setQueryData(key, layout),
    onError: (error) => notifyError(error, { title: t('states.saveError') }),
  })

  const { mutate } = save
  const update = useCallback(
    (layout: SaveLayoutRequest) =>
      mutate({
        widgets: layout.widgets,
        period: layout.period,
        subject: layout.subject ?? null,
        compare_to_previous: layout.compare_to_previous ?? false,
      }),
    [mutate],
  )

  return { update, reset, saving: save.isPending }
}
