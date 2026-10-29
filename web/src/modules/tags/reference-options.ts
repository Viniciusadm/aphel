import { useMemo } from 'react'
import type { ReferenceOptionsContext, ReferenceOptionsProvider, ReferenceOptionsResult } from '@/kernel/modules/define'
import { isCustomRecordType } from '@/features/custom-records/resources'
import { tagsKeys, useTags } from './api'

function useTagOptions({ recordType, enabled = true }: ReferenceOptionsContext): ReferenceOptionsResult {
  const tags = useTags(recordType && !isCustomRecordType(recordType) ? recordType : undefined, { enabled })
  return useMemo(
    () => ({
      options: (tags.data ?? []).map((tag) => ({ value: tag.id, label: tag.name })),
      isLoading: tags.isPending,
    }),
    [tags.data, tags.isPending],
  )
}

export const TAG_REFERENCE_OPTIONS: ReferenceOptionsProvider = { use: useTagOptions, queryKey: tagsKeys.all }
