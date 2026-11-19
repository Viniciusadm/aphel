import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { ToggleField } from '@/components/ui/toggle-field'
import type { Catalog } from '../api'

type Props = { catalog: Catalog; selected: string[]; required: string[]; recommended: string[]; onChange: (keys: string[]) => void }
export function TenantModulePicker({ catalog, selected, required, recommended, onChange }: Props) {
  const { t } = useTranslation('console')
  const { t: moduleText } = useTranslation('modules')
  const [search, setSearch] = useState('')
  const text = (key: string, field: string) => (moduleText as unknown as (key: string, options: object) => string)(`catalog.${key}.${field}`, { defaultValue: '' })
  const name = (key: string) => text(key, 'name') || catalog.modules.find((item) => item.key === key)?.name || key
  const effective = new Set([...selected, ...required])
  let size = -1
  while (size !== effective.size) {
    size = effective.size
    for (const item of catalog.modules) if (effective.has(item.key)) for (const dependency of item.dependencies) effective.add(dependency)
  }
  const items = catalog.modules.filter((item) => `${name(item.key)} ${text(item.key, 'description')}`.toLocaleLowerCase().includes(search.toLocaleLowerCase()))
    .sort((a, b) => Number(effective.has(b.key)) - Number(effective.has(a.key)) || name(a.key).localeCompare(name(b.key)))
  return <div className="grid gap-4">
    <div className="flex flex-wrap items-center justify-between gap-3"><p className="text-sm text-muted-foreground">{t('create.selectionCount', { count: effective.size })}</p>
      <Input className="w-full sm:max-w-xs" aria-label={t('create.searchModules')} placeholder={t('create.searchModules')} value={search} onChange={(event) => setSearch(event.target.value)} /></div>
    <p className="text-sm text-muted-foreground">{t('create.dependenciesHelp')}</p>
    {items.length === 0 && <p role="status" className="py-6 text-center text-muted-foreground">{t('create.noModules')}</p>}
    <div className="grid gap-4 md:grid-cols-2">{items.map((item) => {
      const implied = effective.has(item.key) && !selected.includes(item.key)
      const closure = new Set([item.key])
      let previous = -1
      while (previous !== closure.size) {
        previous = closure.size
        for (const candidate of catalog.modules) if (closure.has(candidate.key)) for (const dependency of candidate.dependencies) closure.add(dependency)
      }
      const needsSettings = catalog.modules.some((candidate) => closure.has(candidate.key) && candidate.requires_configuration)
      const locked = required.includes(item.key) || implied || needsSettings
      const dependents = catalog.modules.filter((other) => effective.has(other.key) && other.dependencies.includes(item.key))
      return <Card key={item.key} className={effective.has(item.key) ? 'border-primary' : undefined}>
        <CardHeader className="gap-2"><CardTitle className="text-base">{name(item.key)}</CardTitle>
          <div className="flex flex-wrap gap-2">{required.includes(item.key) && <Badge variant="secondary">{t('create.requiredModule')}</Badge>}
            {implied && !required.includes(item.key) && <Badge variant="secondary">{t('create.dependency')}</Badge>}
            {recommended.includes(item.key) && <Badge variant="outline">{t('create.recommended')}</Badge>}</div></CardHeader>
        <CardContent className="grid gap-3 text-sm">
          {needsSettings && <p className="text-muted-foreground">{t('create.configureLater')}</p>}
          <p className="text-muted-foreground">{text(item.key, 'description') || t('create.moduleFallback')}</p>
          {text(item.key, 'appears') && <p>{text(item.key, 'appears')}</p>}
          {item.dependencies.length > 0 && <p className="text-muted-foreground">{t('create.dependsOn', { names: item.dependencies.map(name).join(', ') })}</p>}
          {implied && dependents.length > 0 && <p className="text-muted-foreground">{t('create.usedBy', { names: dependents.map((entry) => name(entry.key)).join(', ') })}</p>}
          <ToggleField label={t('create.includeModule', { name: name(item.key) })} checked={effective.has(item.key)} disabled={locked}
            onCheckedChange={(checked) => onChange(checked ? [...selected, item.key] : selected.filter((key) => key !== item.key))} />
        </CardContent>
      </Card>
    })}</div>
  </div>
}
