import { useCallback, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { useNavigate } from 'react-router'
import { usePersonalPreferences } from '@/features/settings/navigation/api'
import { useKeybindingDefinitions, useKeybindingScopes } from './actions'
import { canonicalChord, chordSteps, strokeFromEvent, strokeHasCommandModifier, strokeIsFunctionKey } from './chord'
import { findConflicts } from './conflicts'
import { KeybindingContext, type KeybindingContextValue } from './context'
import { isEditableTarget, isInsideDialog } from './targets'
import { GLOBAL_SCOPE, SEQUENCE_TIMEOUT_MS, type KeybindingAction, type KeybindingHandler, type KeybindingOverrides } from './types'

const EMPTY_OVERRIDES: KeybindingOverrides = {}

interface HandlerEntry {
  handler: { current: KeybindingHandler }
}

interface PendingSequence {
  prefix: string
  timer: number
}

export function KeybindingProvider({ children }: { children: ReactNode }) {
  const navigate = useNavigate()
  const preferences = usePersonalPreferences()
  const user = preferences.data?.keybindings ?? EMPTY_OVERRIDES
  const organization = preferences.data?.keybindings_default ?? EMPTY_OVERRIDES
  const enabled = preferences.data?.keyboard_shortcuts ?? true
  const actions = useKeybindingDefinitions(organization, user)
  const scopes = useKeybindingScopes()
  const [activeScopes, setActiveScopes] = useState<string[]>([])
  const handlers = useRef(new Map<string, HandlerEntry[]>())
  const scopeStack = useRef<string[]>([])
  const pending = useRef<PendingSequence | null>(null)

  const state = useRef({ actions, enabled, navigate })
  useEffect(() => {
    state.current = { actions, enabled, navigate }
  })

  const registerHandler = useCallback((id: string, handler: { current: KeybindingHandler }) => {
    const entry: HandlerEntry = { handler }
    const list = handlers.current.get(id) ?? []
    handlers.current.set(id, [...list, entry])
    return () => {
      const current = handlers.current.get(id) ?? []
      handlers.current.set(
        id,
        current.filter((candidate) => candidate !== entry),
      )
    }
  }, [])

  const activateScope = useCallback((scope: string) => {
    scopeStack.current = [...scopeStack.current, scope]
    setActiveScopes(scopeStack.current)
    return () => {
      const index = scopeStack.current.lastIndexOf(scope)
      if (index >= 0) scopeStack.current = scopeStack.current.filter((_, position) => position !== index)
      setActiveScopes(scopeStack.current)
    }
  }, [])

  useEffect(() => {
    if (!import.meta.env.DEV) return
    const kinds = new Map(scopes.map((scope) => [scope.id, scope.kind]))
    const conflicts = findConflicts(
      actions.map((action) => ({ id: action.id, scope: action.scope, keys: action.defaults })),
      (scope) => {
        const kind = kinds.get(scope)
        return kind === 'global' ? undefined : kind
      },
    ).filter((conflict) => conflict.severity === 'error')
    for (const conflict of conflicts) {
      if (conflict.actionId < (conflict.otherId ?? '')) {
        console.warn(`[keybindings] "${conflict.actionId}" and "${conflict.otherId}" both use "${conflict.chord}"`)
      }
    }
  }, [actions, scopes])

  useEffect(() => {
    const clearPending = () => {
      if (pending.current) window.clearTimeout(pending.current.timer)
      pending.current = null
    }

    const handlerFor = (action: KeybindingAction): KeybindingHandler | null => {
      const registered = handlers.current.get(action.id)?.at(-1)
      if (registered) return registered.handler.current
      if (action.to || action.run) {
        return () => {
          const go = (to: string) => void state.current.navigate(to)
          if (action.to) go(action.to)
          action.run?.({ navigate: go })
        }
      }
      return null
    }

    const candidates = (event: KeyboardEvent, stroke: string) => {
      const { actions: current, enabled: on } = state.current
      const order = [...scopeStack.current].reverse()
      order.push(GLOBAL_SCOPE)
      const editable = isEditableTarget(event.target)
      const dialog = isInsideDialog(event.target)
      const strong = strokeHasCommandModifier(stroke) || strokeIsFunctionKey(stroke)
      const usable = current.filter((action) => {
        if (!order.includes(action.scope)) return false
        if (action.always) return true
        if (!on || dialog) return false
        if (event.repeat && !action.repeat) return false
        if (editable && !action.allowInEditable && !strong) return false
        return true
      })
      return usable.sort((left, right) => order.indexOf(left.scope) - order.indexOf(right.scope))
    }

    const run = (event: KeyboardEvent, matches: KeybindingAction[]) => {
      for (const action of matches) {
        const handler = handlerFor(action)
        if (!handler) continue
        if (handler(event) === false) continue
        event.preventDefault()
        return true
      }
      return false
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.isComposing) return
      const stroke = strokeFromEvent(event)
      if (!stroke) return
      const pool = candidates(event, stroke)
      const keysOf = (action: KeybindingAction) => action.keys.map((key) => canonicalChord(key))

      if (pending.current) {
        const sequence = `${pending.current.prefix} ${stroke}`
        clearPending()
        const matches = pool.filter((action) => keysOf(action).includes(sequence))
        if (run(event, matches)) return
      }

      const exact = pool.filter((action) => keysOf(action).includes(stroke))
      if (run(event, exact)) return
      const startsSequence = pool.some((action) =>
        keysOf(action).some((chord) => {
          const steps = chordSteps(chord)
          return steps.length > 1 && steps[0] === stroke && handlerFor(action) !== null
        }),
      )
      if (startsSequence) {
        event.preventDefault()
        pending.current = { prefix: stroke, timer: window.setTimeout(clearPending, SEQUENCE_TIMEOUT_MS) }
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
      clearPending()
    }
  }, [])

  const value = useMemo<KeybindingContextValue>(
    () => ({
      actions,
      scopes,
      activeScopes,
      enabled,
      organization,
      user,
      registerHandler,
      activateScope,
    }),
    [actions, scopes, activeScopes, enabled, organization, user, registerHandler, activateScope],
  )

  return <KeybindingContext.Provider value={value}>{children}</KeybindingContext.Provider>
}
