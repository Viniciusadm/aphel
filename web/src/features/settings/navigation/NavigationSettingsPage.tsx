import { useId, useMemo, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { ArrowDown, ArrowUp } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { arrangeSidebarEntries, useAvailableSidebarEntries, type SidebarEntry } from '@/app/layout/sidebar-entries'
import { refreshSession } from '@/app/session/session-query'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Checkbox } from '@/components/ui/checkbox'
import { ErrorState } from '@/components/ui/error-state'
import { Label } from '@/components/ui/label'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Skeleton } from '@/components/ui/skeleton'
import { toast } from '@/components/ui/toast-store'
import { SectionHeader } from '@/features/settings/components/SectionHeader'
import { usePersonalPreferences, useUpdatePersonalPreferences } from './api'
import type { NavigationPreferences, PersonalPreferences, SupportedLanguage } from './types'

const FOLLOW_ORGANIZATION = 'organization'
const DEFAULT_HOME = '/'
const LANGUAGES: SupportedLanguage[] = ['pt-BR', 'en']

function LanguageSection({ preferences }: { preferences: PersonalPreferences }) {
  const { t } = useTranslation('navigation-settings')
  const queryClient = useQueryClient()
  const update = useUpdatePersonalPreferences()
  const labelId = useId()

  const change = (value: string) => {
    const locale = value === FOLLOW_ORGANIZATION ? null : (value as SupportedLanguage)
    update.mutate(
      { locale },
      {
        onSuccess: async () => {
          await refreshSession(queryClient)
          toast({ title: t('language.saved'), variant: 'success' })
        },
        onError: (error) => notifyError(error, { title: t('saveError') }),
      },
    )
  }

  return (
    <section className="grid gap-3">
      <SectionHeader title={t('language.title')} description={t('language.description')} />
      <div className="grid max-w-sm gap-1.5">
        <Label id={labelId}>{t('language.label')}</Label>
        <Select value={preferences.locale ?? FOLLOW_ORGANIZATION} onValueChange={change} disabled={update.isPending}>
          <SelectTrigger aria-labelledby={labelId}>
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value={FOLLOW_ORGANIZATION}>{t('language.organization')}</SelectItem>
            {LANGUAGES.map((language) => (
              <SelectItem key={language} value={language}>
                {t(`language.options.${language}`)}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </section>
  )
}

function moveKey(keys: string[], index: number, offset: number): string[] {
  const target = index + offset
  if (target < 0 || target >= keys.length) return keys
  const next = [...keys]
  const [item] = next.splice(index, 1)
  next.splice(target, 0, item)
  return next
}

function NavigationSection({ preferences }: { preferences: PersonalPreferences }) {
  const { t } = useTranslation('navigation-settings')
  const update = useUpdatePersonalPreferences()
  const available = useAvailableSidebarEntries()
  const homeLabelId = useId()
  const [draft, setDraft] = useState<NavigationPreferences>(preferences.navigation)

  const arranged = useMemo(() => arrangeSidebarEntries(available, draft, true), [available, draft])
  const movable = arranged.filter((entry) => !entry.locked)
  const hidden = new Set(draft.hidden)
  const byKey = new Map(available.map((entry) => [entry.key, entry]))
  const homeOptions: SidebarEntry[] = arranged.filter((entry) => entry.to !== DEFAULT_HOME)

  const move = (index: number, offset: number) => {
    const keys = movable.map((entry) => entry.key)
    setDraft({ ...draft, order: moveKey(keys, index, offset) })
  }

  const toggle = (key: string, visible: boolean) => {
    const nextHidden = visible ? draft.hidden.filter((item) => item !== key) : [...draft.hidden, key]
    setDraft({ ...draft, hidden: nextHidden })
  }

  const save = () => {
    const known = draft.order.filter((key) => byKey.has(key))
    const order = known.length > 0 ? movable.map((entry) => entry.key) : []
    update.mutate(
      { navigation: { order, hidden: draft.hidden, home: draft.home } },
      {
        onSuccess: () => toast({ title: t('saved'), variant: 'success' }),
        onError: (error) => notifyError(error, { title: t('saveError') }),
      },
    )
  }

  return (
    <>
      <section className="grid gap-3">
        <SectionHeader title={t('home.title')} description={t('home.description')} />
        <div className="grid max-w-sm gap-1.5">
          <Label id={homeLabelId}>{t('home.label')}</Label>
          <Select
            value={draft.home ?? DEFAULT_HOME}
            onValueChange={(value) => setDraft({ ...draft, home: value === DEFAULT_HOME ? null : value })}
          >
            <SelectTrigger aria-labelledby={homeLabelId}>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value={DEFAULT_HOME}>{t('home.default')}</SelectItem>
              {homeOptions.map((entry) => (
                <SelectItem key={entry.key} value={entry.to}>
                  {entry.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </section>
      <section className="grid gap-3">
        <SectionHeader
          title={t('menu.title')}
          description={t('menu.description')}
          action={
            <Button variant="outline" size="sm" onClick={() => setDraft({ order: [], hidden: [], home: draft.home })}>
              {t('menu.reset')}
            </Button>
          }
        />
        <ul className="grid gap-1 rounded-lg border p-2">
          {movable.map((entry, index) => (
            <li key={entry.key} className="flex items-center gap-3 rounded-md px-2 py-1.5 hover:bg-muted/50">
              <Checkbox
                checked={!hidden.has(entry.key)}
                onCheckedChange={(checked) => toggle(entry.key, checked === true)}
                aria-label={t('menu.visible', { label: entry.label })}
              />
              {entry.icon}
              <span className={hidden.has(entry.key) ? 'flex-1 text-muted-foreground line-through' : 'flex-1'}>
                {entry.label}
              </span>
              <Button
                variant="ghost"
                size="icon"
                disabled={index === 0}
                onClick={() => move(index, -1)}
                aria-label={t('menu.moveUp', { label: entry.label })}
              >
                <ArrowUp />
              </Button>
              <Button
                variant="ghost"
                size="icon"
                disabled={index === movable.length - 1}
                onClick={() => move(index, 1)}
                aria-label={t('menu.moveDown', { label: entry.label })}
              >
                <ArrowDown />
              </Button>
            </li>
          ))}
          {arranged
            .filter((entry) => entry.locked)
            .map((entry) => (
              <li key={entry.key} className="flex items-center gap-3 px-2 py-1.5 text-muted-foreground">
                {entry.icon}
                <span className="flex-1">{entry.label}</span>
                <span className="text-xs">{t('menu.alwaysVisible')}</span>
              </li>
            ))}
        </ul>
        <div>
          <Button onClick={save} disabled={update.isPending}>
            {t('save')}
          </Button>
        </div>
      </section>
    </>
  )
}

export function NavigationSettingsPage() {
  const { t } = useTranslation('navigation-settings')
  const preferences = usePersonalPreferences()

  return (
    <div className="grid gap-8">
      <header className="grid gap-1">
        <h2 className="text-lg font-semibold">{t('title')}</h2>
        <p className="text-sm text-muted-foreground">{t('description')}</p>
      </header>
      {preferences.isPending ? (
        <Skeleton className="h-64" />
      ) : preferences.isError ? (
        <ErrorState onRetry={() => void preferences.refetch()} />
      ) : (
        <>
          <LanguageSection preferences={preferences.data} />
          <NavigationSection key={JSON.stringify(preferences.data.navigation)} preferences={preferences.data} />
        </>
      )}
    </div>
  )
}
