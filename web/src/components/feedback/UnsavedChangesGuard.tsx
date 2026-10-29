import { useUnsavedChangesGuard, type GuardCondition } from './useUnsavedChangesGuard'

export function UnsavedChangesGuard({ when }: { when: GuardCondition }) {
  return useUnsavedChangesGuard(when).dialog
}
