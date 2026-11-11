import { createContext, useCallback, useContext, useEffect, useId, useState, type ReactNode } from 'react'
import { useUnsavedChangesGuard, type GuardCondition } from './useUnsavedChangesGuard'

type Register = (id: string, condition?: GuardCondition) => void
const GuardScope = createContext<Register | null>(null)

export function UnsavedChangesScope({ children }: { children: ReactNode }) {
  const [conditions, setConditions] = useState<Record<string, GuardCondition>>({})
  const register = useCallback<Register>((id, condition) => {
    setConditions((current) => {
      const next = { ...current }
      if (condition === undefined) delete next[id]
      else next[id] = condition
      return next
    })
  }, [])
  const guard = useUnsavedChangesGuard(() => Object.values(conditions).some((condition) => typeof condition === 'function' ? condition() : condition))
  return <GuardScope.Provider value={register}>{children}{guard.dialog}</GuardScope.Provider>
}

function ScopedGuard({ register, when }: { register: Register; when: GuardCondition }) {
  const id = useId()
  useEffect(() => {
    register(id, when)
    return () => register(id)
  }, [id, register, when])
  return null
}

function NavigationGuard({ when }: { when: GuardCondition }) {
  return useUnsavedChangesGuard(when).dialog
}

export function UnsavedChangesGuard({ when }: { when: GuardCondition }) {
  const register = useContext(GuardScope)
  return register ? <ScopedGuard register={register} when={when} /> : <NavigationGuard when={when} />
}
