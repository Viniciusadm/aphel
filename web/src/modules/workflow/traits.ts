import { useQuery } from '@tanstack/react-query'
import { api } from '@/api'
import { getRecordResource } from '@/features/records/resources'
export function isStagedRecord(recordType: string): boolean { return Boolean(getRecordResource(recordType)?.traits?.['workflow.staged']) }
export function stageField(recordType: string): string { const mapping = getRecordResource(recordType)?.traits?.['workflow.staged']; return typeof mapping === 'object' && mapping !== null && 'stage_field' in mapping && typeof mapping.stage_field === 'string' ? mapping.stage_field : 'stage_id' }

export function useStagedFields() {
  return useQuery({ queryKey: ['workflow', 'staged-fields'], queryFn: () => api.get<{ key: string; label: string }[]>('/v1/workflow/required-fields') })
}
