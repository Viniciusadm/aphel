import { useMemo } from 'react'
import type { NavContribution } from './define'
import { useLabelTranslator } from './i18n'
import { contributions } from './registry'
import type { SettingsEntry } from './settings'

export interface ModuleSurface {
  hasScreens: boolean
  menu: string[]
  settings: string[]
}

function unique(values: string[]): string[] {
  return [...new Set(values.filter((value) => value !== ''))]
}

export function moduleHasScreens(key: string): boolean {
  return contributions<NavContribution>('nav').some((item) => item.module === key)
}

export function useModuleSurface(key: string): ModuleSurface {
  const translate = useLabelTranslator()
  return useMemo(() => {
    const menu = contributions<NavContribution>('nav')
      .filter((item) => item.module === key)
      .map((item) => translate(key, item.value.label))
    const settings = contributions<SettingsEntry>('settings')
      .filter((item) => item.module === key)
      .map((item) => translate(key, item.value.item.label))
    return { hasScreens: menu.length > 0, menu: unique(menu), settings: unique(settings) }
  }, [key, translate])
}
