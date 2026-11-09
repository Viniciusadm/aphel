import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { SearchInput } from '@/components/ui/search-input'
import {
  chordText,
  conflictsForChord,
  findConflicts,
  normalizeKeys,
  resolveKeys,
  useKeybindings,
  type KeybindingAction,
  type KeybindingConflict,
  type KeybindingLayer,
  type KeybindingOverrides,
  type KeybindingScopeKind,
  type KeybindingSource,
  KeybindingKeys,
} from '@/kernel/keybindings'
import { KeyRecorderDialog, type RecordedChoice } from './KeyRecorderDialog'

export interface KeybindingsEditorProps {
  layer: KeybindingLayer
  overrides: KeybindingOverrides
  pending: boolean
  onSave: (next: KeybindingOverrides) => Promise<unknown>
}

interface LayerAction extends KeybindingAction {
  layerKeys: string[]
  layerSource: KeybindingSource
}

function matches(query: string, ...values: Array<string | undefined>) {
  const needle = query.trim().toLocaleLowerCase()
  if (!needle) return true
  return values.some((value) => value?.toLocaleLowerCase().includes(needle))
}

export function KeybindingsEditor({ layer, overrides, pending, onSave }: KeybindingsEditorProps) {
  const { t } = useTranslation('keybindings')
  const { actions, scopes, organization } = useKeybindings()
  const [query, setQuery] = useState('')
  const [editing, setEditing] = useState<LayerAction | null>(null)

  const layerActions = useMemo<LayerAction[]>(
    () =>
      actions.map((action) => {
        const resolved =
          layer === 'organization'
            ? resolveKeys(action.id, action.defaults, overrides, {})
            : resolveKeys(action.id, action.defaults, organization, overrides)
        return { ...action, layerKeys: resolved.keys, layerSource: resolved.source }
      }),
    [actions, layer, overrides, organization],
  )

  const scopeKinds = useMemo(() => new Map(scopes.map((scope) => [scope.id, scope.kind])), [scopes])
  const scopeKind = (scope: string): KeybindingScopeKind | undefined => {
    const kind = scopeKinds.get(scope)
    return kind === 'global' ? undefined : kind
  }
  const candidates = layerActions.map((action) => ({ id: action.id, scope: action.scope, keys: action.layerKeys }))
  const conflicts = findConflicts(candidates, scopeKind)
  const errorsById = new Map<string, KeybindingConflict[]>()
  for (const conflict of conflicts) {
    if (conflict.severity !== 'error') continue
    errorsById.set(conflict.actionId, [...(errorsById.get(conflict.actionId) ?? []), conflict])
  }

  const labelOf = (id: string | undefined) => layerActions.find((action) => action.id === id)?.label ?? id ?? ''
  const scopeLabel = (id: string | undefined) => scopes.find((scope) => scope.id === id)?.label ?? id ?? ''
  const describeConflict = (conflict: KeybindingConflict) => {
    const other = layerActions.find((action) => action.id === conflict.otherId)
    const values = {
      keys: chordText(conflict.chord),
      otherKeys: conflict.otherChord ? chordText(conflict.otherChord) : '',
      action: labelOf(conflict.actionId),
      other: labelOf(conflict.otherId),
      scope: scopeLabel(other?.scope),
    }
    return t(`conflicts.${conflict.kind}`, values)
  }

  const persist = async (next: KeybindingOverrides) => {
    try {
      await onSave(next)
      notifySuccess(t('saved'))
    } catch (error) {
      notifyError(error, { title: t('saveError') })
    }
  }

  const withoutOverride = (id: string): KeybindingOverrides => {
    const next = { ...overrides }
    delete next[id]
    return next
  }

  const setKeys = (current: KeybindingOverrides, action: LayerAction, keys: string[]): KeybindingOverrides => {
    const normalized = normalizeKeys(keys)
    const inherited = layer === 'organization' ? action.defaults : resolveKeys(action.id, action.defaults, organization, {}).keys
    const next = { ...current }
    if (normalized.join('\n') === inherited.join('\n')) delete next[action.id]
    else next[action.id] = normalized
    return next
  }

  const save = (choice: RecordedChoice) => {
    if (!editing) return
    const keys = choice.append ? [...editing.layerKeys, choice.chord] : [choice.chord]
    let next = setKeys(overrides, editing, keys)
    if (choice.resolveConflicts) {
      const blocking = conflictsForChord(editing.id, editing.scope, choice.chord, candidates, scopeKind).filter(
        (conflict) => conflict.severity === 'error',
      )
      for (const conflict of blocking) {
        const other = layerActions.find((action) => action.id === conflict.otherId)
        if (!other) continue
        next = setKeys(next, other, other.layerKeys.filter((key) => key !== conflict.otherChord))
      }
    }
    void persist(next).then(() => setEditing(null))
  }

  const sections = scopes
    .map((scope) => ({
      scope,
      rows: layerActions.filter(
        (action) =>
          action.scope === scope.id &&
          matches(query, action.label, action.description, action.id, ...action.layerKeys.map((key) => chordText(key))),
      ),
    }))
    .filter((section) => section.rows.length > 0)

  const errorCount = new Set(conflicts.filter((conflict) => conflict.severity === 'error').map((conflict) => conflict.actionId)).size
  const customized = Object.keys(overrides).length > 0

  const sourceLabel: Record<KeybindingSource, string> = {
    default: t('source.default'),
    organization: t('source.organization'),
    user: t('source.user'),
  }

  return (
    <div className="grid gap-6">
      <div className="flex flex-wrap items-center gap-3">
        <SearchInput
          value={query}
          onValueChange={setQuery}
          placeholder={t('search')}
          aria-label={t('search')}
          wrapperClassName="min-w-0 flex-1 basis-64"
        />
        {customized && (
          <Button variant="outline" disabled={pending} onClick={() => void persist({})}>
            {t('resetAll')}
          </Button>
        )}
      </div>

      {errorCount > 0 && (
        <div role="alert" className="rounded-md border border-destructive/30 bg-destructive/5 px-3 py-2 text-sm">
          {t('conflictSummary', { count: errorCount })}
        </div>
      )}

      {sections.length === 0 && <p className="text-sm text-muted-foreground">{t('empty')}</p>}

      {sections.map(({ scope, rows }) => (
        <section key={scope.id} className="grid gap-2" aria-labelledby={`keybindings-scope-${scope.id}`}>
          <div className="grid gap-0.5">
            <h2 id={`keybindings-scope-${scope.id}`} className="text-sm font-semibold">
              {scope.label}
            </h2>
            {scope.description && <p className="text-sm text-muted-foreground">{scope.description}</p>}
          </div>
          <ul className="divide-y rounded-lg border">
            {rows.map((action) => {
              const actionErrors = errorsById.get(action.id) ?? []
              const overridden = Object.hasOwn(overrides, action.id)
              return (
                <li key={action.id} className="flex flex-wrap items-center gap-x-4 gap-y-2 px-4 py-3">
                  <div className="grid min-w-0 flex-1 basis-56 gap-0.5">
                    <span className="text-sm font-medium">{action.label}</span>
                    {action.description && <span className="text-sm text-muted-foreground">{action.description}</span>}
                    {actionErrors.map((conflict) => (
                      <span key={`${conflict.kind}:${conflict.otherId}`} className="text-sm text-destructive">
                        {describeConflict(conflict)}
                      </span>
                    ))}
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    {action.layerKeys.length > 0 ? (
                      <KeybindingKeys keys={action.layerKeys} />
                    ) : (
                      <span className="text-sm text-muted-foreground">{t('none')}</span>
                    )}
                    <Badge tone={action.layerSource === 'default' ? 'neutral' : 'primary'} size="sm">
                      {sourceLabel[action.layerSource]}
                    </Badge>
                  </div>
                  <div className="flex flex-wrap items-center gap-1">
                    <Button size="sm" variant="outline" disabled={pending} onClick={() => setEditing(action)}>
                      {t('change')}
                    </Button>
                    {action.layerKeys.length > 0 && !action.always && (
                      <Button size="sm" variant="ghost" disabled={pending} onClick={() => void persist(setKeys(overrides, action, []))}>
                        {t('disable')}
                      </Button>
                    )}
                    {overridden && (
                      <Button size="sm" variant="ghost" disabled={pending} onClick={() => void persist(withoutOverride(action.id))}>
                        {t('reset')}
                      </Button>
                    )}
                  </div>
                </li>
              )
            })}
          </ul>
        </section>
      ))}

      <KeyRecorderDialog
        action={editing}
        currentKeys={editing?.layerKeys ?? []}
        pending={pending}
        conflictsFor={(chord) => (editing ? conflictsForChord(editing.id, editing.scope, chord, candidates, scopeKind) : [])}
        describeConflict={describeConflict}
        onSave={save}
        onOpenChange={(open) => {
          if (!open) setEditing(null)
        }}
      />
    </div>
  )
}

