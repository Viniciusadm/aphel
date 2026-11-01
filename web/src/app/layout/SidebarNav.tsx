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

const ITEM_CLASS =
  'relative flex h-9 items-center gap-3 rounded-md px-3 text-sm font-medium text-sidebar-foreground transition-colors hover:bg-sidebar-accent hover:text-sidebar-accent-foreground focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground'

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
    <NavLink
      to={entry.to}
      end={entry.end}
      onClick={onNavigate}
      aria-label={collapsed ? entry.label : undefined}
      className={cn(ITEM_CLASS, collapsed && 'justify-center px-0', nested && !collapsed && 'h-8 pl-9')}
    >
      {entry.icon}
      {!collapsed && <span className="truncate">{entry.label}</span>}
      {entry.badge && <entry.badge collapsed={collapsed} />}
    </NavLink>
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
      <button
        type="button"
        aria-expanded={expanded}
        aria-controls={contentId}
        onClick={() => setDisclosure({ path: location.pathname, open: !expanded })}
        className={cn(ITEM_CLASS, 'w-full text-left')}
      >
        {node.icon}
        <span className="flex-1 truncate">{node.label}</span>
        <ChevronDown className={cn('size-4 shrink-0 transition-transform', !expanded && '-rotate-90')} aria-hidden />
      </button>
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
  const preferences = useNavigationPreferences()
  const nodes = groupEntries(arrangeSidebarEntries(useAvailableSidebarEntries(), preferences))

  return (
    <nav aria-label={t('nav.label')} className="flex flex-col gap-1 px-2">
      {nodes.map((node) =>
        node.kind === 'entry' ? (
          <EntryLink key={node.entry.key} entry={node.entry} collapsed={collapsed} onNavigate={onNavigate} />
        ) : (
          <GroupSection key={node.key} node={node} collapsed={collapsed} onNavigate={onNavigate} />
        ),
      )}
    </nav>
  )
}
