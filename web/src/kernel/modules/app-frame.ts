import type { ReactNode } from 'react'
import type { CommandPaletteControls } from '@/features/command-palette/context'
import type { NavigationEntry } from './navigation'
import type { SettingsArea } from './settings'

export const APP_FRAME_CONTRACT = 'app.Frame'
export const DEFAULT_APP_FRAME = 'kernel'

export interface AppFrameProps {
  children: ReactNode
  navigation: NavigationEntry[]
  commands: CommandPaletteControls
  settingsArea: SettingsArea | null
  returnPath: string
  mobileNavOpen: boolean
  onMobileNavOpenChange: (open: boolean) => void
}

declare module './contract-components' {
  interface ContractComponentMap {
    'app.Frame': AppFrameProps
  }
}
