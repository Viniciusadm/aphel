import { useCallback, useEffect, useMemo, useRef, type KeyboardEvent as ReactKeyboardEvent } from 'react'
import { canonicalChord, chordSteps, strokeFromEvent } from './chord'
import { useKeybindings } from './context'
import type { KeybindingAction, KeybindingHandler } from './types'

export interface UseKeybindingOptions {
  enabled?: boolean
}

export function useKeybinding(id: string, handler: KeybindingHandler, options: UseKeybindingOptions = {}) {
  const { registerHandler } = useKeybindings()
  const enabled = options.enabled ?? true
  const ref = useRef(handler)
  useEffect(() => {
    ref.current = handler
  })
  useEffect(() => {
    if (!enabled) return
    return registerHandler(id, ref)
  }, [id, enabled, registerHandler])
}

export function useKeybindingHandlers(handlers: Record<string, KeybindingHandler>, options: UseKeybindingOptions = {}) {
  const { registerHandler } = useKeybindings()
  const enabled = options.enabled ?? true
  const ref = useRef(handlers)
  useEffect(() => {
    ref.current = handlers
  })
  const ids = Object.keys(handlers).sort().join('\n')
  useEffect(() => {
    if (!enabled || ids === '') return
    const releases = ids.split('\n').map((id) =>
      registerHandler(id, {
        get current(): KeybindingHandler {
          return (event) => ref.current[id]?.(event)
        },
      }),
    )
    return () => releases.forEach((release) => release())
  }, [ids, enabled, registerHandler])
}

export function useKeybindingScope(scope: string, enabled = true) {
  const { activateScope } = useKeybindings()
  useEffect(() => {
    if (!enabled) return
    return activateScope(scope)
  }, [scope, enabled, activateScope])
}

export function useKeybindingAction(id: string): KeybindingAction | undefined {
  const { actions } = useKeybindings()
  return useMemo(() => actions.find((action) => action.id === id), [actions, id])
}

export function useActionKeys(id: string): string[] {
  return useKeybindingAction(id)?.keys ?? []
}

export function useKeybindingProps<E extends Element = Element>(handlers: Record<string, KeybindingHandler>) {
  const { actions } = useKeybindings()
  const ref = useRef(handlers)
  useEffect(() => {
    ref.current = handlers
  })
  const onKeyDown = useCallback(
    (event: ReactKeyboardEvent<E>) => {
      if (event.defaultPrevented || event.nativeEvent.isComposing) return
      const stroke = strokeFromEvent(event.nativeEvent)
      if (!stroke) return
      for (const [id, handler] of Object.entries(ref.current)) {
        const action = actions.find((candidate) => candidate.id === id)
        if (!action) continue
        const matches = action.keys.some((key) => {
          const chord = canonicalChord(key)
          return chordSteps(chord).length === 1 && chord === stroke
        })
        if (!matches) continue
        if (event.repeat && !action.repeat) continue
        if (handler(event.nativeEvent) === false) continue
        event.preventDefault()
        event.stopPropagation()
        return
      }
    },
    [actions],
  )
  return { onKeyDown }
}
