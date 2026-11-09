import { useMemo } from 'react'
import { useTranslation } from 'react-i18next'
import type { KeybindingContribution, KeybindingScopeContribution } from '@/kernel/modules/define'
import { useLabelTranslator } from '@/kernel/modules/i18n'
import { useActiveContributions, usePermissionCheck } from '@/kernel/modules/registry'
import { normalizeChord } from './chord'
import {
  GLOBAL_SCOPE,
  type KeybindingAction,
  type KeybindingOverrides,
  type KeybindingScope,
  type KeybindingSource,
} from './types'

export function normalizeKeys(keys: readonly string[]): string[] {
  const result: string[] = []
  for (const key of keys) {
    const chord = normalizeChord(key)
    if (chord && !result.includes(chord)) result.push(chord)
  }
  return result
}

export function resolveKeys(
  id: string,
  defaults: string[],
  organization: KeybindingOverrides,
  user: KeybindingOverrides,
): { keys: string[]; source: KeybindingSource } {
  if (Object.hasOwn(user, id)) return { keys: normalizeKeys(user[id]), source: 'user' }
  if (Object.hasOwn(organization, id)) return { keys: normalizeKeys(organization[id]), source: 'organization' }
  return { keys: defaults, source: 'default' }
}

export function useKeybindingScopes(): KeybindingScope[] {
  const items = useActiveContributions<KeybindingScopeContribution>('keybindingScopes')
  const translate = useLabelTranslator()
  const { t } = useTranslation('keybindings')
  return useMemo(
    () => [
      { id: GLOBAL_SCOPE, module: 'kernel', label: t('scopes.global'), description: t('scopes.globalDescription'), kind: 'global' as const },
      ...items.map(({ module, value }) => ({
        id: value.id,
        module,
        label: translate(module, value.label),
        description: value.description ? translate(module, value.description) : undefined,
        kind: value.kind,
      })),
    ],
    [items, translate, t],
  )
}

export function useKeybindingDefinitions(
  organization: KeybindingOverrides,
  user: KeybindingOverrides,
): KeybindingAction[] {
  const items = useActiveContributions<KeybindingContribution>('keybindings')
  const allowed = usePermissionCheck()
  const translate = useLabelTranslator()
  const { t } = useTranslation('keybindings')
  return useMemo(() => {
    const seen = new Set<string>()
    const actions: KeybindingAction[] = []
    for (const { module, value } of items) {
      if (seen.has(value.id) || !allowed(value.permission)) continue
      seen.add(value.id)
      const defaults = normalizeKeys(value.keys)
      const label = translate(module, value.label)
      const { keys, source } = resolveKeys(value.id, defaults, organization, user)
      actions.push({
        id: value.id,
        module,
        label: value.to ? t('goTo', { page: label }) : label,
        description: value.description ? translate(module, value.description) : undefined,
        scope: value.scope ?? GLOBAL_SCOPE,
        defaults,
        keys,
        source,
        allowInEditable: value.allowInEditable ?? false,
        always: value.always ?? false,
        repeat: value.repeat ?? false,
        to: value.to,
        run: value.run,
      })
    }
    return actions
  }, [items, allowed, translate, t, organization, user])
}
