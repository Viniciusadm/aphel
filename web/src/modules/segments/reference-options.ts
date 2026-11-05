import { useMemo } from 'react'
import type { ReferenceOptionsContext, ReferenceOptionsProvider, ReferenceOptionsResult } from '@/kernel/modules/define'
import {segmentsKeys, useSegments} from '@/sdk'

function useSegmentReferenceOptions({ recordType, enabled = true }: ReferenceOptionsContext): ReferenceOptionsResult {
  const segments = useSegments(recordType ?? '', { enabled: enabled && Boolean(recordType) })
  return useMemo(
    () => ({
      options: (segments.data ?? []).map((segment) => ({ value: segment.id, label: segment.name })),
      isLoading: segments.isPending,
    }),
    [segments.data, segments.isPending],
  )
}

export const SEGMENT_REFERENCE_OPTIONS: ReferenceOptionsProvider = {
  use: useSegmentReferenceOptions,
  queryKey: segmentsKeys.all,
}
