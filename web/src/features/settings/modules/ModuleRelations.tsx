import { useTranslation } from 'react-i18next'
import { Badge } from '@/components/ui/badge'
import type { ModuleInfo } from '@/kernel/modules/api'
import { moduleAnchor } from './chain'

type Link = { key: string; enabled: boolean }

interface ModuleRelationsProps {
  module: ModuleInfo
  nameOf: (key: string) => string
  exists: (key: string) => boolean
}

function jump(key: string) {
  const target = document.getElementById(moduleAnchor(key))
  if (!target) return
  target.scrollIntoView({ behavior: 'smooth', block: 'center' })
  target.focus({ preventScroll: true })
}

export function ModuleRelations({ module, nameOf, exists }: ModuleRelationsProps) {
  const { t } = useTranslation('modules')
  const requires: Link[] = module.dependencies.map((dependency) => ({
    key: dependency.key,
    enabled: dependency.installed && dependency.enabled,
  }))
  const required = new Set(requires.map((link) => link.key))
  const prefers: Link[] = module.optional_dependencies
    .filter((link) => !required.has(link.key))
    .map((link) => ({ key: link.key, enabled: link.enabled }))
  const usedBy: Link[] = module.dependents.map((link) => ({ key: link.key, enabled: link.enabled }))

  const rows = [
    { id: 'requires', label: t('relations.requires'), links: requires, warnWhenOff: true },
    { id: 'prefers', label: t('relations.prefers'), links: prefers, warnWhenOff: false },
    { id: 'usedBy', label: t('relations.usedBy'), links: usedBy, warnWhenOff: false },
  ].filter((row) => row.links.length > 0)

  if (rows.length === 0) return null

  return (
    <dl className="grid gap-2 text-xs">
      {rows.map((row) => (
        <div key={row.id} className="grid gap-1">
          <dt className="font-medium text-muted-foreground">{row.label}</dt>
          <dd>
            <ul className="flex flex-wrap gap-1.5">
              {row.links.map((link) => {
                const state = link.enabled ? 'enabled' : 'disabled'
                const tone = link.enabled ? 'success' : row.warnWhenOff ? 'warning' : 'neutral'
                const content = (
                  <Badge tone={tone}>
                    {nameOf(link.key)}
                    <span className="font-normal">· {t(`relations.state.${state}`)}</span>
                  </Badge>
                )
                return (
                  <li key={link.key}>
                    {exists(link.key) ? (
                      <button type="button" className="rounded-md focus-visible:outline-2" onClick={() => jump(link.key)}>
                        {content}
                      </button>
                    ) : (
                      content
                    )}
                  </li>
                )
              })}
            </ul>
          </dd>
        </div>
      ))}
    </dl>
  )
}
