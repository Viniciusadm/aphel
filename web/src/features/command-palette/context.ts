import { createContext, useContext, useEffect, useId } from 'react'
import type { CreateRequest, PaletteCommand, PaletteMode } from './types'

export interface CommandPaletteControls {
  openPalette: (mode?: PaletteMode) => void
  closePalette: () => void
  togglePalette: () => void
  openCreate: (request: CreateRequest) => void
  openHelp: () => void
  registerCommands: (owner: string, commands: PaletteCommand[]) => void
  unregisterCommands: (owner: string) => void
}

export const CommandPaletteControlsContext = createContext<CommandPaletteControls | null>(null)

export function useCommandPalette(): CommandPaletteControls {
  const value = useContext(CommandPaletteControlsContext)
  if (!value) throw new Error('useCommandPalette must be used inside CommandPaletteProvider')
  return value
}

export function useRegisterCommands(commands: PaletteCommand[]) {
  const controls = useContext(CommandPaletteControlsContext)
  const owner = useId()

  useEffect(() => {
    if (!controls) return
    controls.registerCommands(owner, commands)
  }, [controls, owner, commands])

  useEffect(() => {
    if (!controls) return
    return () => controls.unregisterCommands(owner)
  }, [controls, owner])
}
