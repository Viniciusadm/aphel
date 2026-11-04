import { useEffect, useRef } from 'react'

export const SEQUENCE_TIMEOUT_MS = 1200

export type GoShortcutKey = 'h' | 'c' | 'e' | 'd' | 'p' | 't' | 'i' | 'a' | 'r' | 's'

export interface GoShortcut {
  key: GoShortcutKey
  to: string
}

export const GO_SHORTCUTS: GoShortcut[] = [
  { key: 'h', to: '/' },
  { key: 'c', to: '/people' },
  { key: 'e', to: '/organizations' },
  { key: 'd', to: '/deals' },
  { key: 'p', to: '/catalog-items' },
  { key: 't', to: '/tasks' },
  { key: 'i', to: '/inbox' },
  { key: 'a', to: '/calendar' },
  { key: 'r', to: '/reports' },
  { key: 's', to: '/settings' },
]

export interface ShortcutHandlers {
  enabled: boolean
  onTogglePalette: () => void
  onCreate: () => void
  onHelp: () => void
  onGo: (shortcut: GoShortcut) => void
}

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false
  if (target.isContentEditable) return true
  const tag = target.tagName
  if (tag === 'INPUT' || tag === 'TEXTAREA' || tag === 'SELECT') return true
  return Boolean(target.closest('[contenteditable=""], [contenteditable="true"], [role="textbox"]'))
}

export function isPaletteShortcut(event: KeyboardEvent): boolean {
  return (event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'k'
}

function hasModifier(event: KeyboardEvent): boolean {
  return event.ctrlKey || event.metaKey || event.altKey
}

function isInsideDialog(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && Boolean(target.closest('[role="dialog"], [role="alertdialog"], [role="menu"]'))
}

export function useGlobalShortcuts(handlers: ShortcutHandlers) {
  const handlersRef = useRef(handlers)
  const pendingGoRef = useRef<number | null>(null)

  useEffect(() => {
    handlersRef.current = handlers
  })

  useEffect(() => {
    const clearPending = () => {
      if (pendingGoRef.current !== null) window.clearTimeout(pendingGoRef.current)
      pendingGoRef.current = null
    }

    const onKeyDown = (event: KeyboardEvent) => {
      const current = handlersRef.current
      if (event.defaultPrevented || event.isComposing) return
      if (isPaletteShortcut(event)) {
        event.preventDefault()
        clearPending()
        current.onTogglePalette()
        return
      }
      if (!current.enabled || hasModifier(event) || event.repeat) return
      if (isEditableTarget(event.target) || isInsideDialog(event.target)) {
        clearPending()
        return
      }
      const key = event.key.toLowerCase()
      if (pendingGoRef.current !== null) {
        clearPending()
        const shortcut = GO_SHORTCUTS.find((candidate) => candidate.key === key)
        if (shortcut) {
          event.preventDefault()
          current.onGo(shortcut)
        }
        return
      }
      if (event.key === '?') {
        event.preventDefault()
        current.onHelp()
        return
      }
      if (event.shiftKey) return
      if (key === 'g') {
        event.preventDefault()
        pendingGoRef.current = window.setTimeout(clearPending, SEQUENCE_TIMEOUT_MS)
        return
      }
      if (key === 'c') {
        event.preventDefault()
        current.onCreate()
      }
    }

    window.addEventListener('keydown', onKeyDown)
    return () => {
      window.removeEventListener('keydown', onKeyDown)
      clearPending()
    }
  }, [])
}
