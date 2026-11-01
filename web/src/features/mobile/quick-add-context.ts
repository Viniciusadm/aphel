import { createContext, useContext } from 'react'
import type { ModuleQuickAdd } from '@/kernel/modules/quick-add'

export interface MobileQuickAddContextValue {
  actions: ModuleQuickAdd[]
  open: (mode: string) => void
}

export const MobileQuickAddContext = createContext<MobileQuickAddContextValue | null>(null)

export function useMobileQuickAdd() {
  return useContext(MobileQuickAddContext)
}
