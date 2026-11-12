import { CornerDownRight, Plus, Trash2 } from 'lucide-react'
import { useQueryClient } from '@tanstack/react-query'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess, UnsavedChangesGuard } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { NativeSelect } from '@/components/ui/native-select'
import { Skeleton } from '@/components/ui/skeleton'
import { type OrganizationConfiguration, type SiteNavigationItem, SortableList } from '@/sdk'
import { saveNavigation, type SiteTarget, useInvalidateSite, useSiteTargets } from './api'
import { currentSettings } from './siteSettings'

const ADDRESS = '__address__'
const GROUP = '__group__'
const MAX_ITEMS = 30
const MAX_CHILDREN = 15
const LABEL_MAX = 60

function newId(): string {
  return `nav-${crypto.randomUUID().slice(0, 8)}`
}

function grouped(targets: SiteTarget[]): [string, SiteTarget[]][] {
  const groups = new Map<string, SiteTarget[]>()
  for (const target of targets) groups.set(target.group, [...(groups.get(target.group) ?? []), target])
  return [...groups.entries()]
}

function TargetSelect({ id, value, targets, allowAddress, allowGroup, emptyLabel, disabled, onChange }: {
  id: string
  value: string
  targets: SiteTarget[]
  allowAddress?: boolean
  allowGroup?: boolean
  emptyLabel?: string
  disabled?: boolean
  onChange: (value: string) => void
}) {
  const { t } = useTranslation('site')
  const known = value === '' || value === ADDRESS || value === GROUP || targets.some((target) => target.target === value)
  return (
    <NativeSelect id={id} className="w-full" value={value} disabled={disabled} onValueChange={onChange}>
      {emptyLabel !== undefined && <option value="">{emptyLabel}</option>}
      {!known && <option value={value}>{t('navigation.missingTarget')}</option>}
      {grouped(targets).map(([group, items]) => (
        <optgroup key={group} label={group}>
          {items.map((target) => (
            <option key={target.target} value={target.target}>
              {target.label} ({target.path})
            </option>
          ))}
        </optgroup>
      ))}
      {allowAddress && <option value={ADDRESS}>{t('navigation.address')}</option>}
      {allowGroup && <option value={GROUP}>{t('navigation.group')}</option>}
    </NativeSelect>
  )
}

function choiceOf(item: SiteNavigationItem): string {
  if (item.target) return item.target
  if (item.url !== null && item.url !== undefined) return ADDRESS
  return GROUP
}

function ItemEditor({ item, targets, nested, disabled, onChange, onRemove, onAddChild }: {
  item: SiteNavigationItem
  targets: SiteTarget[]
  nested: boolean
  disabled: boolean
  onChange: (item: SiteNavigationItem) => void
  onRemove: () => void
  onAddChild?: () => void
}) {
  const { t } = useTranslation('site')
  const choice = choiceOf(item)
  const target = targets.find((candidate) => candidate.target === item.target)
  return (
    <div className="grid flex-1 gap-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1.4fr)_auto] sm:items-end">
      <Field label={t('navigation.label')} description={target && !item.label ? t('navigation.labelFallback', { label: target.label }) : undefined}>
        {(control) => (
          <Input {...control} value={item.label} maxLength={LABEL_MAX} placeholder={target?.label} disabled={disabled} onChange={(event) => onChange({ ...item, label: event.target.value })} />
        )}
      </Field>
      <Field label={t('navigation.points')}>
        {(control) => (
          <TargetSelect
            id={control.id}
            value={choice}
            targets={targets}
            allowAddress
            allowGroup={!nested}
            disabled={disabled}
            onChange={(value) =>
              onChange({
                ...item,
                target: value === ADDRESS || value === GROUP ? null : value,
                url: value === ADDRESS ? (item.url ?? '') : null,
              })
            }
          />
        )}
      </Field>
      <div className="flex gap-1">
        {onAddChild && (
          <Button type="button" size="icon" variant="ghost" disabled={disabled || item.children.length >= MAX_CHILDREN} aria-label={t('navigation.addChild', { label: item.label || target?.label || '' })} onClick={onAddChild}>
            <CornerDownRight aria-hidden="true" />
          </Button>
        )}
        <Button type="button" size="icon" variant="ghost" disabled={disabled} aria-label={t('navigation.remove', { label: item.label || target?.label || '' })} onClick={onRemove}>
          <Trash2 aria-hidden="true" />
        </Button>
      </div>
      {choice === ADDRESS && (
        <Field label={t('navigation.url')} description={t('navigation.urlHelp')} className="sm:col-span-3">
          {(control) => (
            <Input {...control} value={item.url ?? ''} placeholder="/contato" inputMode="url" spellCheck={false} disabled={disabled} onChange={(event) => onChange({ ...item, url: event.target.value })} />
          )}
        </Field>
      )}
    </div>
  )
}

function cleaned(items: SiteNavigationItem[]): SiteNavigationItem[] {
  return items.map((item) => ({
    id: item.id,
    label: item.label.trim(),
    target: item.target || null,
    url: item.target ? null : item.url === null || item.url === undefined ? null : item.url.trim(),
    children: cleaned(item.children),
  }))
}

export function NavigationCards({ configuration }: { configuration: OrganizationConfiguration }) {
  const { t } = useTranslation('site')
  const targets = useSiteTargets()
  const client = useQueryClient()
  const invalidate = useInvalidateSite()
  const settings = currentSettings(configuration)
  const [items, setItems] = useState<SiteNavigationItem[]>(() => settings.navigation ?? [])
  const [home, setHome] = useState(settings.home ?? '')
  const [saved, setSaved] = useState(() => JSON.stringify([settings.navigation ?? [], settings.home ?? '']))
  const [saving, setSaving] = useState(false)
  const dirty = JSON.stringify([items, home]) !== saved
  const list = useMemo(() => targets.data ?? [], [targets.data])

  const update = (id: string, change: (item: SiteNavigationItem) => SiteNavigationItem) =>
    setItems((current) =>
      current.map((item) => (item.id === id ? change(item) : { ...item, children: item.children.map((child) => (child.id === id ? change(child) : child)) })),
    )
  const remove = (id: string) =>
    setItems((current) => current.filter((item) => item.id !== id).map((item) => ({ ...item, children: item.children.filter((child) => child.id !== id) })))
  const reorder = (ids: string[], within?: string) =>
    setItems((current) => {
      if (!within) return ids.map((id) => current.find((item) => item.id === id)).filter((item): item is SiteNavigationItem => item !== undefined)
      return current.map((item) =>
        item.id === within ? { ...item, children: ids.map((id) => item.children.find((child) => child.id === id)).filter((child): child is SiteNavigationItem => child !== undefined) } : item,
      )
    })
  const blank = (): SiteNavigationItem => {
    const used = new Set([home, ...items.flatMap((item) => [item.target, ...item.children.map((child) => child.target)])])
    const depth = (target: SiteTarget) => target.path.split('/').filter(Boolean).length
    const suggested = list.find((target) => !used.has(target.target) && depth(target) <= 1)
    return { id: newId(), label: '', target: suggested?.target ?? null, url: suggested ? null : '', children: [] }
  }

  const save = async () => {
    setSaving(true)
    try {
      const navigation = cleaned(items)
      await saveNavigation(navigation, home || null)
      await client.invalidateQueries({ queryKey: ['organization', 'configuration'] })
      setItems(navigation)
      setSaved(JSON.stringify([navigation, home]))
      void invalidate()
      notifySuccess(t('navigation.saved'))
    } catch (error) {
      notifyError(error)
    } finally {
      setSaving(false)
    }
  }

  if (targets.isPending) return <Skeleton className="h-64" />

  return (
    <div className="grid gap-6">
      <UnsavedChangesGuard when={dirty} />
      <Card>
        <CardHeader>
          <CardTitle>{t('navigation.homeTitle')}</CardTitle>
          <CardDescription>{t('navigation.homeDescription')}</CardDescription>
        </CardHeader>
        <CardContent>
          <Field label={t('navigation.home')}>
            {(control) => <TargetSelect id={control.id} value={home} targets={list} emptyLabel={t('navigation.homeDefault')} disabled={saving} onChange={setHome} />}
          </Field>
        </CardContent>
      </Card>
      <Card>
        <CardHeader>
          <CardTitle>{t('navigation.menuTitle')}</CardTitle>
          <CardDescription>{t('navigation.menuDescription')}</CardDescription>
        </CardHeader>
        <CardContent className="grid gap-4">
          {items.length === 0 ? (
            <EmptyState title={t('navigation.empty')} />
          ) : (
            <SortableList
              items={items}
              getId={(item) => item.id}
              getLabel={(item) => item.label || list.find((target) => target.target === item.target)?.label || t('navigation.untitled')}
              disabled={saving}
              label={t('navigation.listLabel')}
              onReorder={(ids) => reorder(ids)}
              renderItem={(item, handle) => (
                <div className="grid gap-3 rounded-md border bg-card px-3 py-3">
                  <div className="flex items-start gap-3">
                    <span className="pt-7">{handle}</span>
                    <ItemEditor
                      item={item}
                      targets={list}
                      nested={false}
                      disabled={saving}
                      onChange={(next) => update(item.id, () => next)}
                      onRemove={() => remove(item.id)}
                      onAddChild={() => update(item.id, (current) => ({ ...current, children: [...current.children, blank()] }))}
                    />
                  </div>
                  {item.children.length > 0 && (
                    <div className="ml-8 border-l pl-4">
                      <SortableList
                        items={item.children}
                        getId={(child) => child.id}
                        getLabel={(child) => child.label || list.find((target) => target.target === child.target)?.label || t('navigation.untitled')}
                        disabled={saving}
                        label={t('navigation.childrenLabel', { label: item.label })}
                        onReorder={(ids) => reorder(ids, item.id)}
                        renderItem={(child, childHandle) => (
                          <div className="flex items-start gap-3 py-2">
                            <span className="pt-7">{childHandle}</span>
                            <ItemEditor item={child} targets={list} nested disabled={saving} onChange={(next) => update(child.id, () => next)} onRemove={() => remove(child.id)} />
                          </div>
                        )}
                      />
                    </div>
                  )}
                </div>
              )}
            />
          )}
          <div className="flex flex-wrap justify-between gap-2">
            <Button type="button" variant="outline" disabled={saving || items.length >= MAX_ITEMS} onClick={() => setItems((current) => [...current, blank()])}>
              <Plus aria-hidden="true" />
              {t('navigation.add')}
            </Button>
            <Button type="button" disabled={saving || !dirty} onClick={() => void save()}>
              {t('navigation.save')}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  )
}
