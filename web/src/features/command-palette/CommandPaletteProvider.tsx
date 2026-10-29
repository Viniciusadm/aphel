import { useCallback, useMemo, useState, type ReactNode } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { useGlobalShortcuts, type GoShortcut } from '@/app/shortcuts'
import { useNavigationEntries } from '@/kernel/modules/navigation'
import { lazyComponent } from '@/lib/lazy-component'
import { CommandPaletteControlsContext, type CommandPaletteControls } from './context'
import { contextualCreateAction, useCreateActions } from './create-actions'
import { CreateRecordDialogs } from './CreateRecordDialogs'
import { usePreferences } from './preferences/api'
import { ShortcutsHelpDialog } from './ShortcutsHelpDialog'
import type { CreateRequest, PaletteCommand, PaletteMode } from './types'

const CommandPalette = lazyComponent(async () => (await import('./CommandPalette')).CommandPalette)

export function CommandPaletteProvider({ children }: { children: ReactNode }) {
  const navigate = useNavigate()
  const location = useLocation()
  const navigation = useNavigationEntries()
  const preferences = usePreferences()
  const createActions = useCreateActions()
  const [paletteMode, setPaletteMode] = useState<PaletteMode | null>(null)
  const [paletteMounted, setPaletteMounted] = useState(false)
  const [createRequest, setCreateRequest] = useState<CreateRequest | null>(null)
  const [helpOpen, setHelpOpen] = useState(false)
  const [registry, setRegistry] = useState<Map<string, PaletteCommand[]>>(() => new Map())

  if (paletteMode !== null && !paletteMounted) setPaletteMounted(true)

  const registerCommands = useCallback((owner: string, commands: PaletteCommand[]) => {
    setRegistry((current) => {
      const next = new Map(current)
      next.set(owner, commands)
      return next
    })
  }, [])

  const unregisterCommands = useCallback((owner: string) => {
    setRegistry((current) => {
      if (!current.has(owner)) return current
      const next = new Map(current)
      next.delete(owner)
      return next
    })
  }, [])

  const controls = useMemo<CommandPaletteControls>(
    () => ({
      openPalette: (mode = 'all') => setPaletteMode(mode),
      closePalette: () => setPaletteMode(null),
      togglePalette: () => setPaletteMode((current) => (current ? null : 'all')),
      openCreate: (request) => {
        setPaletteMode(null)
        setCreateRequest(request)
      },
      openHelp: () => {
        setPaletteMode(null)
        setHelpOpen(true)
      },
      registerCommands,
      unregisterCommands,
    }),
    [registerCommands, unregisterCommands],
  )

  const contextCommands = useMemo(() => [...registry.values()].flat(), [registry])
  const busy = createRequest !== null || helpOpen

  const goTo = useCallback(
    (shortcut: GoShortcut) => {
      if (!navigation.some((entry) => entry.to === shortcut.to)) return
      navigate(shortcut.to)
    },
    [navigation, navigate],
  )

  const createInContext = useCallback(() => {
    const action = contextualCreateAction(location.pathname, createActions)
    if (action) controls.openCreate({ kind: action.key })
    else if (createActions.length > 0) controls.openPalette('create')
  }, [controls, createActions, location.pathname])

  useGlobalShortcuts({
    enabled: (preferences.data?.keyboard_shortcuts ?? true) && !busy && paletteMode === null,
    onTogglePalette: () => {
      if (!busy) controls.togglePalette()
    },
    onCreate: createInContext,
    onHelp: controls.openHelp,
    onGo: goTo,
  })

  return (
    <CommandPaletteControlsContext.Provider value={controls}>
      {children}
      {(paletteMounted || paletteMode !== null) && (
        <CommandPalette
          mode={paletteMode}
          onModeChange={setPaletteMode}
          contextCommands={contextCommands}
          createActions={createActions}
        />
      )}
      <CreateRecordDialogs actions={createActions} request={createRequest} onClose={() => setCreateRequest(null)} />
      <ShortcutsHelpDialog
        open={helpOpen}
        onOpenChange={setHelpOpen}
        enabled={preferences.data?.keyboard_shortcuts ?? true}
      />
    </CommandPaletteControlsContext.Provider>
  )
}
