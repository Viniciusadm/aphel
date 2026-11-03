import { SearchInput } from '@/components/ui/search-input'
import { SidebarLink } from '@/components/ui/sidebar-link'
import { ChevronDown } from 'lucide-react'
import { useId, useState, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { NavLink, useLocation } from 'react-router'
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip'
import { useNavigationPreferences } from '@/features/settings/navigation/api'
import { cn } from '@/lib/utils'
import { arrangeSidebarEntries, useAvailableSidebarEntries, type SidebarEntry } from './sidebar-entries'

interface SidebarNavProps {
  collapsed?: boolean
  onNavigate?: () => void
}

type SidebarNode =
  | { kind: 'entry'; entry: SidebarEntry }
  | { kind: 'group'; key: string; label: string; icon: ReactNode; entries: SidebarEntry[] }


function groupEntries(entries: SidebarEntry[]): SidebarNode[] {
  const nodes: SidebarNode[] = []
  const groups = new Map<string, Extract<SidebarNode, { kind: 'group' }>>()
  for (const entry of entries) {
    if (!entry.group) {
      nodes.push({ kind: 'entry', entry })
      continue
    }
    const existing = groups.get(entry.group.key)
    if (existing) {
      existing.entries.push(entry)
      continue
    }
    const node: Extract<SidebarNode, { kind: 'group' }> = {
      kind: 'group',
      key: entry.group.key,
      label: entry.group.label,
      icon: entry.group.icon,
      entries: [entry],
    }
    groups.set(entry.group.key, node)
    nodes.push(node)
  }
  return nodes
}

function EntryLink({
  entry,
  collapsed,
  nested = false,
  onNavigate,
}: {
  entry: SidebarEntry
  collapsed: boolean
  nested?: boolean
  onNavigate?: () => void
}) {
  const link = (
    <SidebarLink asChild collapsed={collapsed} nested={nested}>
      <NavLink
        to={entry.to}
        end={entry.end}
        onClick={onNavigate}
        aria-label={collapsed ? entry.label : undefined}
      >
        {entry.icon}
        {!collapsed && <span className="truncate">{entry.label}</span>}
        {entry.badge && <entry.badge collapsed={collapsed} />}
      </NavLink>
    </SidebarLink>
  )
  if (!collapsed) return link
  return (
    <Tooltip>
      <TooltipTrigger asChild>{link}</TooltipTrigger>
      <TooltipContent side="right">{entry.label}</TooltipContent>
    </Tooltip>
  )
}

function GroupSection({
  node,
  collapsed,
  onNavigate,
}: {
  node: Extract<SidebarNode, { kind: 'group' }>
  collapsed: boolean
  onNavigate?: () => void
}) {
  const location = useLocation()
  const active = node.entries.some((entry) => location.pathname === entry.to || location.pathname.startsWith(`${entry.to}/`))
  const contentId = useId()
  const [disclosure, setDisclosure] = useState<{ path: string; open: boolean } | null>(null)
  if (collapsed) {
    return (
      <>
        {node.entries.map((entry) => (
          <EntryLink key={entry.key} entry={entry} collapsed onNavigate={onNavigate} />
        ))}
      </>
    )
  }
  const expanded = disclosure?.path === location.pathname ? disclosure.open : active
  return (
    <div className="flex flex-col gap-1">
      <SidebarLink asChild className="w-full text-left">
        <button
          type="button"
          aria-expanded={expanded}
          aria-controls={contentId}
          onClick={() => setDisclosure({ path: location.pathname, open: !expanded })}
        >
          {node.icon}
          <span className="flex-1 truncate">{node.label}</span>
          <ChevronDown className={cn('size-4 shrink-0 transition-transform', !expanded && '-rotate-90')} aria-hidden />
        </button>
      </SidebarLink>
      {expanded && <div id={contentId} className="grid gap-1">
        {node.entries.map((entry) => (
          <EntryLink key={entry.key} entry={entry} collapsed={false} nested onNavigate={onNavigate} />
        ))}
      </div>}
    </div>
  )
}

export function SidebarNav({ collapsed = false, onNavigate }: SidebarNavProps) {
  const { t } = useTranslation('layout')
  const [query, setQuery] = useState('')
  const preferences = useNavigationPreferences()
  const entries = arrangeSidebarEntries(useAvailableSidebarEntries(), preferences)
  const filtered = entries.filter((entry) => !query || `${entry.label} ${entry.group?.label ?? ''}`.toLocaleLowerCase().includes(query.toLocaleLowerCase()))
  const categories = ['work', 'crm', 'operations', 'analysis', 'administration'] as const
  const category = (entry: SidebarEntry) => {
    if (entry.locked || /^\/(settings|system|sso|google|microsoft)(\/|$)/.test(entry.to)) return 'administration'
    if (entry.to === '/calls/report') return 'analysis'
    if (/^\/(reports|forecast|goals)(\/|$)/.test(entry.to)) return 'analysis'
    if (/^\/(people|organizations|leads|deals|objects)(\/|$)/.test(entry.to) || entry.key.startsWith('object:')) return 'crm'
    if (/^\/(today|tasks|calendar|inbox|messaging|following|search)(\/|$)/.test(entry.to) || entry.to === '/') return 'work'
    return 'operations'
  }

  return (
    <nav aria-label={t('nav.label')} className="flex flex-col gap-1 px-2">
      {!collapsed && <SearchInput value={query} onValueChange={setQuery} aria-label={t('nav.search')} placeholder={t('nav.search')} wrapperClassName="mb-3" />}
      {categories.map((key) => {
        const nodes = groupEntries(filtered.filter((entry) => category(entry) === key))
        if (nodes.length === 0) return null
        return <section key={key} className="grid gap-1 pb-4">
          {!collapsed && <h2 className="px-3 pb-1 text-xs font-semibold text-muted-foreground">{t(`nav.groups.${key}`)}</h2>}
          {nodes.map((node) => node.kind === 'entry' ? <EntryLink key={node.entry.key} entry={node.entry} collapsed={collapsed} onNavigate={onNavigate} /> : <GroupSection key={node.key} node={node} collapsed={collapsed} onNavigate={onNavigate} />)}
        </section>
      })}

    </nav>
  )
}
