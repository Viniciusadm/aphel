import { useMutation, useQueryClient } from '@tanstack/react-query'
import { api } from '@/api'
import type { KeybindingOverrides } from '@/kernel/keybindings'
import { useUpdatePersonalPreferences } from '@/features/settings/navigation/api'

const PREFERENCES_QUERY_KEY = ['preferences'] as const

export function useSaveUserKeybindings() {
  const update = useUpdatePersonalPreferences()
  return {
    isPending: update.isPending,
    save: (keybindings: KeybindingOverrides) => update.mutateAsync({ keybindings }),
  }
}

export function useSaveOrganizationKeybindings() {
  const queryClient = useQueryClient()
  const mutation = useMutation({
    mutationFn: (keybindings: KeybindingOverrides) => api.put('/v1/organization/keybindings', { keybindings }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: PREFERENCES_QUERY_KEY }),
  })
  return { isPending: mutation.isPending, save: mutation.mutateAsync }
}
