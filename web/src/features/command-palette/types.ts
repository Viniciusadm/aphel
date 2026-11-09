import type { LucideIcon } from 'lucide-react'
import type { CreateTarget } from '@/kernel/modules/define'

export type { CreateTarget }

export type CommandGroup = 'context' | 'create' | 'favorites' | 'navigation' | 'general'

export interface PaletteCommand {
  id: string
  label: string
  group: CommandGroup
  icon?: LucideIcon
  keywords?: string[]
  keybinding?: string
  run: () => void
}

export type CreateKind = string

export interface CreateRequest {
  kind: CreateKind
  target?: CreateTarget
}

export type PaletteMode = 'all' | 'create'
