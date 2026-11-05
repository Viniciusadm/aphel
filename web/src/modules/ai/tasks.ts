import { useQueryClient } from '@tanstack/react-query'
import { activitiesKeys } from '@/modules/tasks/api'
import { ACTIVITY_RECORD_TYPE } from '@/modules/tasks/resource'
import { recordsKeys } from '@/sdk'
export function useInvalidateActivitiesForRecord(recordType: string, recordId: string) {
  const queryClient = useQueryClient()
  return () =>
    Promise.all([
      queryClient.invalidateQueries({ queryKey: activitiesKeys.all }),
      queryClient.invalidateQueries({ queryKey: [...recordsKeys.all, ACTIVITY_RECORD_TYPE] }),
      queryClient.invalidateQueries({ queryKey: recordsKeys.detail(recordType, recordId) }),
    ])
}
