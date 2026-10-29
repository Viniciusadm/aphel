import { useQueryClient } from '@tanstack/react-query'
import { activitiesKeys } from '@/modules/activities/api'
import { ACTIVITY_RECORD_TYPE } from '@/modules/activities/resource'
import { recordsKeys } from '@/features/records/api'

export function useInvalidateActivitiesForRecord(recordType: string, recordId: string) {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: activitiesKeys.all }),
      queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, ACTIVITY_RECORD_TYPE] }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.detail(recordType, recordId) }),
    ])
}
