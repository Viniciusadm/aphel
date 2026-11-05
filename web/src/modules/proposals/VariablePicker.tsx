import { Braces } from 'lucide-react'
import { useMemo, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import {PROPOSALS_KEY, type ProposalVariable} from '@/sdk'

const GROUP_ORDER = ['deal', 'contact', 'company', 'owner', 'items', 'blocks', 'proposal', 'user', 'organization']

interface VariablePickerProps {
  variables: ProposalVariable[]
  disabled?: boolean
  onPick: (variable: ProposalVariable) => void
}

export function VariablePicker({ variables, disabled, onPick }: VariablePickerProps) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const [open, setOpen] = useState(false)
  const [search, setSearch] = useState('')

  const groups = useMemo(() => {
    const term = search.trim().toLowerCase()
    const matches = variables.filter(
      (variable) =>
        !term || variable.key.toLowerCase().includes(term) || variable.label.toLowerCase().includes(term),
    )
    const keys = [...new Set(matches.map((variable) => variable.group))].sort(
      (left, right) => rank(left) - rank(right),
    )
    return keys.map((group) => ({ group, items: matches.filter((variable) => variable.group === group) }))
  }, [variables, search])

  return (
    <Popover
      open={open}
      onOpenChange={(next) => {
        setOpen(next)
        if (!next) setSearch('')
      }}
    >
      <PopoverTrigger asChild>
        <Button type="button" size="sm" variant="outline" disabled={disabled}>
          <Braces aria-hidden="true" />
          {t('editor.insertVariable')}
        </Button>
      </PopoverTrigger>
      <PopoverContent align="start" className="w-[min(22rem,calc(100vw-2rem))] p-0">
        <div className="border-b p-2">
          <Input
            value={search}
            autoFocus
            onChange={(event) => setSearch(event.target.value)}
            placeholder={t('editor.searchVariable')}
            aria-label={t('editor.searchVariable')}
          />
        </div>
        <div className="max-h-80 overflow-y-auto p-1">
          {groups.length === 0 ? (
            <p className="px-2 py-3 text-sm text-muted-foreground">{t('editor.noVariables')}</p>
          ) : (
            groups.map(({ group, items }) => (
              <div key={group} role="group" aria-label={groupLabel(t, group)} className="py-1">
                <p className="px-2 py-1 text-xs font-medium text-muted-foreground">{groupLabel(t, group)}</p>
                {items.map((variable) => (
                  <button
                    key={variable.key}
                    type="button"
                    className="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent focus-visible:bg-accent focus-visible:outline-none"
                    onClick={() => {
                      onPick(variable)
                      setOpen(false)
                      setSearch('')
                    }}
                  >
                    <span className="min-w-0 truncate">{variable.label}</span>
                    <code className="shrink-0 truncate text-xs text-muted-foreground">
                      {variable.block ? t('editor.block') : variable.key}
                    </code>
                  </button>
                ))}
              </div>
            ))
          )}
        </div>
      </PopoverContent>
    </Popover>
  )
}

function rank(group: string): number {
  const index = GROUP_ORDER.indexOf(group)
  return index === -1 ? GROUP_ORDER.length : index
}

function groupLabel(t: (key: string) => string, group: string): string {
  return GROUP_ORDER.includes(group) ? t(`groups.${group}`) : group
}
