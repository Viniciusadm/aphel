import { useQueryClient } from '@tanstack/react-query'
import {activitiesKeys} from '@/sdk'
import {ACTIVITY_RECORD_TYPE} from '@/sdk'
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
