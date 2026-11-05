import { useIsModuleActive } from '@/kernel/modules/registry'
import { ArrowLeft, CornerDownLeft, Keyboard, Loader2, Search, SlidersHorizontal, type LucideIcon } from 'lucide-react'
import { useCallback, useEffect, useId, useMemo, useRef, useState, type KeyboardEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { GO_SHORTCUTS } from '@/app/shortcuts'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import { recordDetailHref, recordTypeIcon } from '@/features/records/links'
import { SEARCH_MIN_LENGTH, useGlobalSearch, type SearchGroup } from '@/modules/search/ui'
import { NO_FILTER_CONTEXT, useFilters } from '@/kernel/modules/filters'
import { useModuleCommands } from '@/kernel/modules/contributions'
import { qualifiedLabel, useNavigationEntries } from '@/kernel/modules/navigation'
import { cn } from '@/lib/utils'
import { useRecordTypeOptions } from '@/features/settings/fields/lib/record-types'
import { useCommandPalette } from './context'
import type { CreateAction } from './create-actions'
import { matchesQuery } from './matching'
import type { CommandGroup, PaletteCommand, PaletteMode } from './types'

const SEARCH_DEBOUNCE_MS = 200
const SEARCH_LIMIT = 5
const PREFERENCES_PATH = '/account/preferences'
const SEARCH_PATH = '/search'
const GROUP_ORDER: CommandGroup[] = ['context', 'create', 'favorites', 'navigation', 'general']

interface PaletteOption {
  id: string
  label: string
  description?: string
  icon?: LucideIcon
  shortcut?: string[]
  run: () => void
}

interface PaletteSection {
  id: string
  label: string
  options: PaletteOption[]
}

export interface CommandPaletteProps {
  mode: PaletteMode | null
  onModeChange: (mode: PaletteMode | null) => void
  contextCommands: PaletteCommand[]
  createActions: CreateAction[]
}

function useDebouncedValue(value: string, delay: number) {
  const [debounced, setDebounced] = useState(value)
  useEffect(() => {
    const timer = window.setTimeout(() => setDebounced(value), delay)
    return () => window.clearTimeout(timer)
  }, [value, delay])
  return debounced
}

function Kbd({ children }: { children: string }) {
  return (
    <kbd className="inline-flex h-5 min-w-5 items-center justify-center rounded border bg-muted px-1 font-mono text-[11px] text-muted-foreground">
      {children}
    </kbd>
  )
}

function commandOption(command: PaletteCommand): PaletteOption {
  return { id: command.id, label: command.label, icon: command.icon, shortcut: command.shortcut, run: command.run }
}

function filterCommands(commands: PaletteCommand[], query: string): PaletteCommand[] {
  return commands.filter((command) => matchesQuery(query, command.label, ...(command.keywords ?? [])))
}

export function CommandPalette({ mode, onModeChange, contextCommands, createActions }: CommandPaletteProps) {
  const open = mode !== null
  return (
    <Dialog open={open} onOpenChange={(next) => onModeChange(next ? (mode ?? 'all') : null)}>
      <DialogContent size="lg"
        showCloseButton={false}
        className="top-[8vh] max-h-[80vh] translate-y-0 [--dialog-gap:0px] [--dialog-padding:0px] sm:top-[12vh]">
        {open && (
          <PaletteBody
            mode={mode}
            onModeChange={onModeChange}
            contextCommands={contextCommands}
            createActions={createActions}
          />
        )}
      </DialogContent>
    </Dialog>
  )
}

interface PaletteBodyProps extends CommandPaletteProps {
  mode: PaletteMode
}

function PaletteBody({ mode, onModeChange, contextCommands, createActions }: PaletteBodyProps) {
  const { t } = useTranslation('command-palette')
  const navigate = useNavigate()
  const navigationEntries = useNavigationEntries()
  const moduleCommands = useModuleCommands()
  const controls = useCommandPalette()
  const [query, setQuery] = useState('')
  const [activeIndex, setActiveIndex] = useState(0)
  const debouncedQuery = useDebouncedValue(query, SEARCH_DEBOUNCE_MS)
  const searchActive = useIsModuleActive('search')
  const searchEnabled = searchActive && mode === 'all'
  const search = useGlobalSearch({ q: searchEnabled ? debouncedQuery : '', limit: SEARCH_LIMIT })
  const listRef = useRef<HTMLDivElement>(null)
  const baseId = useId()
  const listId = `${baseId}-list`
  const trimmed = query.trim()

  const changeMode = (next: PaletteMode) => {
    setActiveIndex(0)
    onModeChange(next)
  }

  const execute = (run: () => void) => {
    onModeChange(null)
    window.setTimeout(run, 0)
  }

  const entityOptions = useRecordTypeOptions()
  const recordTypeLabel = useCallback(
    (recordType: string) => entityOptions.find((option) => option.value === recordType)?.label ?? recordType,
    [entityOptions],
  )

  const baseCommands = useMemo<PaletteCommand[]>(() => {
    const create: PaletteCommand[] = createActions.map((action) => ({
      id: `create:${action.key}`,
      label: action.label,
      group: 'create',
      icon: action.icon,
      keywords: [t('groups.create')],
      run: () => controls.openCreate({ kind: action.key }),
    }))
    const navigation: PaletteCommand[] = navigationEntries.map((item) => {
      const page = qualifiedLabel(item, navigationEntries)
      const shortcut = GO_SHORTCUTS.find((candidate) => candidate.to === item.to)
      return {
        id: `nav:${item.to}`,
        label: t('commands.goTo', { page }),
        group: 'navigation',
        icon: item.icon,
        keywords: [page],
        shortcut: shortcut ? ['G', shortcut.key.toUpperCase()] : undefined,
        run: () => navigate(item.to),
      }
    })
    const contributed: PaletteCommand[] = moduleCommands.map((command) => ({
      id: `${command.module}:${command.id}`,
      label: command.label,
      group: command.to ? 'navigation' : 'general',
      icon: command.icon,
      keywords: command.keywords,
      run: () => {
        if (command.to) navigate(command.to)
        command.run?.({ navigate: (to) => void navigate(to) })
      },
    }))
    const general: PaletteCommand[] = [
      {
        id: 'general:shortcuts',
        label: t('commands.shortcuts'),
        group: 'general',
        icon: Keyboard,
        shortcut: ['?'],
        run: controls.openHelp,
      },
      {
        id: 'general:preferences',
        label: t('commands.preferences'),
        group: 'general',
        icon: SlidersHorizontal,
        run: () => navigate(PREFERENCES_PATH),
      },
    ]
    return [...contextCommands, ...create, ...navigation, ...contributed, ...general]
  }, [contextCommands, controls, createActions, moduleCommands, navigate, navigationEntries, t])
  const commands = useFilters('commands', baseCommands, NO_FILTER_CONTEXT)

  const settled = searchEnabled && trimmed.length >= SEARCH_MIN_LENGTH && debouncedQuery.trim() === trimmed
  const searchGroups = useMemo<SearchGroup[]>(
    () => (settled ? (search.data?.groups ?? []) : []),
    [settled, search.data],
  )

  const sections = useMemo<PaletteSection[]>(() => {
    const groupLabels: Record<CommandGroup, string> = {
      context: t('groups.context'),
      create: t('groups.create'),
      favorites: t('groups.favorites'),
      navigation: t('groups.navigation'),
      general: t('groups.general'),
    }
    const visible = mode === 'create' ? commands.filter((command) => command.group === 'create') : commands
    const filtered = filterCommands(visible, trimmed)
    const resultSections: PaletteSection[] = searchGroups
      .filter((group) => group.items.length > 0)
      .map((group) => ({
        id: `search:${group.record_type}`,
        label: t('groups.results', { type: recordTypeLabel(group.record_type) }),
        options: group.items.flatMap((hit) => {
          const href = recordDetailHref(hit.record_type, hit.id)
          if (!href) return []
          return [
            {
              id: `hit:${hit.record_type}:${hit.id}`,
              label: hit.title,
              description: hit.subtitle ?? undefined,
              icon: recordTypeIcon(hit.record_type),
              run: () => navigate(href),
            },
          ]
        }),
      }))
    const commandSections = GROUP_ORDER
      .map((group) => ({
        id: `group:${group}`,
        label: groupLabels[group],
        options: filtered.filter((command) => command.group === group).map(commandOption),
      }))
      .filter((section) => section.options.length > 0)
    if (searchEnabled && trimmed.length >= SEARCH_MIN_LENGTH) {
      resultSections.push({
        id: 'search:all',
        label: t('groups.search'),
        options: [
          {
            id: 'search:all-results',
            label: t('commands.allResults', { query: trimmed }),
            icon: Search,
            run: () => navigate(`${SEARCH_PATH}?${new URLSearchParams({ q: trimmed }).toString()}`),
          },
        ],
      })
    }
    return [...resultSections, ...commandSections]
  }, [commands, mode, navigate, recordTypeLabel, searchEnabled, searchGroups, t, trimmed])

  const options = useMemo(() => sections.flatMap((section) => section.options), [sections])
  const optionDomId = (index: number) => `${baseId}-option-${index}`
  const safeIndex = options.length === 0 ? -1 : Math.min(activeIndex, options.length - 1)

  useEffect(() => {
    if (safeIndex < 0) return
    const element = listRef.current?.querySelector<HTMLElement>(`[data-index="${safeIndex}"]`)
    element?.scrollIntoView({ block: 'nearest' })
  }, [safeIndex])

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault()
      if (options.length === 0) return
      const delta = event.key === 'ArrowDown' ? 1 : -1
      setActiveIndex((safeIndex + delta + options.length) % options.length)
      return
    }
    if (event.key === 'Home' && event.ctrlKey) {
      event.preventDefault()
      setActiveIndex(0)
      return
    }
    if (event.key === 'End' && event.ctrlKey) {
      event.preventDefault()
      setActiveIndex(Math.max(options.length - 1, 0))
      return
    }
    if (event.key === 'Enter') {
      event.preventDefault()
      const option = options[safeIndex]
      if (option) execute(option.run)
      return
    }
    if (event.key === 'Backspace' && query === '' && mode === 'create') {
      event.preventDefault()
      changeMode('all')
    }
  }

  const searching = searchEnabled && trimmed.length >= SEARCH_MIN_LENGTH && (search.isFetching || debouncedQuery.trim() !== trimmed)
  const showMinLength = searchEnabled && trimmed.length > 0 && trimmed.length < SEARCH_MIN_LENGTH
  let optionIndex = -1

  return (
    <>
      <DialogTitle className="sr-only">{t('dialog.title')}</DialogTitle>
      <DialogDescription className="sr-only">{t('dialog.description')}</DialogDescription>
      <div className="flex items-center gap-2 border-b px-3">
        {mode === 'create' ? (
          <button
            type="button"
            onClick={() => changeMode('all')}
            className="flex size-7 shrink-0 items-center justify-center rounded-md text-muted-foreground hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
            aria-label={t('dialog.back')}
          >
            <ArrowLeft className="size-4" aria-hidden />
          </button>
        ) : searching ? (
          <Loader2 className="size-4 shrink-0 animate-spin text-muted-foreground" aria-hidden />
        ) : (
          <Search className="size-4 shrink-0 text-muted-foreground" aria-hidden />
        )}
        <input
          autoFocus
          value={query}
          onChange={(event) => {
            setQuery(event.target.value)
            setActiveIndex(0)
          }}
          onKeyDown={onKeyDown}
          role="combobox"
          aria-expanded="true"
          aria-controls={listId}
          aria-autocomplete="list"
          aria-activedescendant={safeIndex >= 0 ? optionDomId(safeIndex) : undefined}
          aria-label={t('dialog.inputLabel')}
          placeholder={mode === 'create' ? t('dialog.createPlaceholder') : t('dialog.placeholder')}
          className="h-12 min-w-0 flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
          autoComplete="off"
          spellCheck={false}
        />
      </div>
      <div
        ref={listRef}
        id={listId}
        role="listbox"
        aria-label={t('dialog.listLabel')}
        className="max-h-[min(60vh,28rem)] overflow-y-auto overscroll-contain p-2"
      >
        {sections.map((section) => {
          const headingId = `${baseId}-${section.id}`
          return (
            <div key={section.id} role="group" aria-labelledby={headingId} className="pb-2 last:pb-0">
              <div id={headingId} className="px-2 pt-1 pb-1 text-xs font-medium text-muted-foreground">
                {section.label}
              </div>
              {section.options.map((option) => {
                optionIndex += 1
                const index = optionIndex
                const active = index === safeIndex
                const Icon = option.icon
                return (
                  <div
                    key={option.id}
                    id={optionDomId(index)}
                    data-index={index}
                    role="option"
                    aria-selected={active}
                    onMouseMove={() => {
                      if (!active) setActiveIndex(index)
                    }}
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => execute(option.run)}
                    className={cn(
                      'flex min-h-9 cursor-pointer items-center gap-3 rounded-md px-2 py-1.5 text-sm',
                      active ? 'bg-accent text-accent-foreground' : 'text-foreground',
                    )}
                  >
                    {Icon && <Icon className="size-4 shrink-0 text-muted-foreground" aria-hidden />}
                    <span className="grid min-w-0 flex-1">
                      <span className="truncate">{option.label}</span>
                      {option.description && (
                        <span className="truncate text-xs text-muted-foreground">{option.description}</span>
                      )}
                    </span>
                    {option.shortcut && (
                      <span className="hidden shrink-0 items-center gap-1 sm:flex" aria-hidden>
                        {option.shortcut.map((key) => (
                          <Kbd key={key}>{key}</Kbd>
                        ))}
                      </span>
                    )}
                    {active && <CornerDownLeft className="size-3.5 shrink-0 text-muted-foreground" aria-hidden />}
                  </div>
                )
              })}
            </div>
          )
        })}
        {options.length === 0 && !searching && (
          <p className="px-2 py-6 text-center text-sm text-muted-foreground" role="status">
            {t('dialog.empty', { query: trimmed })}
          </p>
        )}
      </div>
      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 border-t px-3 py-2 text-xs text-muted-foreground">
        <span role="status" aria-live="polite" className="min-w-0 flex-1 truncate">
          {searching
            ? t('dialog.searching')
            : search.isError && searchEnabled
              ? t('dialog.searchError')
              : showMinLength
                ? t('dialog.minLength', { count: SEARCH_MIN_LENGTH })
                : ''}
        </span>
        <span className="hidden items-center gap-1 sm:flex">
          <Kbd>↑</Kbd>
          <Kbd>↓</Kbd>
          {t('dialog.hint.navigate')}
        </span>
        <span className="hidden items-center gap-1 sm:flex">
          <Kbd>↵</Kbd>
          {t('dialog.hint.select')}
        </span>
        <span className="hidden items-center gap-1 sm:flex">
          <Kbd>Esc</Kbd>
          {t('dialog.hint.close')}
        </span>
      </div>
    </>
  )
}
