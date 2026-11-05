import type { LucideIcon } from 'lucide-react'
import { useMemo, type ComponentType } from 'react'
import { useEntities } from '@/sdk/entities'
import type { TraitListViewContribution, TraitListViewProps } from './define'
import { useLabelTranslator } from './i18n'
import { sortContributions, useActiveContributions } from './registry'

export interface ResolvedTraitListView {
  key: string
  label: string
  icon: LucideIcon
  traitConfig: unknown
  render: ComponentType<TraitListViewProps>
}

export function useTraitListViews(recordType: string): ResolvedTraitListView[] {
  const items = useActiveContributions<TraitListViewContribution>('traitListViews')
  const entities = useEntities()
  const translate = useLabelTranslator()
  return useMemo(() => {
    const traits = entities.data?.find((entity) => entity.record_type === recordType)?.traits ?? {}
    return sortContributions(items)
      .filter(({ value }) => Boolean(traits[value.trait]))
      .map(({ module, value }) => ({
        key: value.key,
        label: translate(module, value.label),
        icon: value.icon,
        traitConfig: traits[value.trait],
        render: value.render,
      }))
  }, [items, entities.data, recordType, translate])
}
