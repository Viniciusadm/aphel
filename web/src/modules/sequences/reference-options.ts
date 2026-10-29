import { useMemo } from 'react'
import type { ReferenceOptionsContext, ReferenceOptionsProvider, ReferenceOptionsResult } from '@/kernel/modules/define'
import { sequencesKeys, useActiveSequences } from './api'

function useSequenceOptions({ enabled = true }: ReferenceOptionsContext): ReferenceOptionsResult {
  const sequences = useActiveSequences(enabled)
  return useMemo(
    () => ({
      options: (sequences.data ?? []).map((sequence) => ({ value: sequence.id, label: sequence.name })),
      isLoading: sequences.isPending,
    }),
    [sequences.data, sequences.isPending],
  )
}

export const SEQUENCE_REFERENCE_OPTIONS: ReferenceOptionsProvider = { use: useSequenceOptions, queryKey: sequencesKeys.all }
