import { useMemo } from 'react'
import type { ConfigPackageLabels } from './define'
import { useLabelTranslator } from './i18n'
import { useActiveContributions } from './registry'

export interface ResolvedConfigPackageLabels {
  sections: Record<string, string>
  kinds: Record<string, string>
  properties: Record<string, string>
}

function translated(
  module: string,
  labels: Record<string, string> | undefined,
  translate: (module: string, label: string) => string,
): Record<string, string> {
  return Object.fromEntries(Object.entries(labels ?? {}).map(([key, label]) => [key, translate(module, label)]))
}

export function useModuleConfigPackageLabels(): ResolvedConfigPackageLabels {
  const items = useActiveContributions<ConfigPackageLabels>('configPackageLabels')
  const translate = useLabelTranslator()
  return useMemo(
    () =>
      items.reduce<ResolvedConfigPackageLabels>(
        (result, { module, value }) => ({
          sections: { ...result.sections, ...translated(module, value.sections, translate) },
          kinds: { ...result.kinds, ...translated(module, value.kinds, translate) },
          properties: { ...result.properties, ...translated(module, value.properties, translate) },
        }),
        { sections: {}, kinds: {}, properties: {} },
      ),
    [items, translate],
  )
}
