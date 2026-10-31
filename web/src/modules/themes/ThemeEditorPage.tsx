import { SegmentedControl } from '@/components/ui/segmented-control'
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { ArrowLeft, Save } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useLocation, useNavigate, useParams } from 'react-router'
import { isApiError } from '@/api'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Skeleton } from '@/components/ui/skeleton'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { Textarea } from '@/components/ui/textarea'
import { RequirePermission } from '@/features/settings/components/RequirePermission'
import { themesApi, themesKeys } from './api'
import { FontsEditor } from './FontsEditor'
import { contrastWarnings } from './lib/contrast-report'
import { THEME_KEY_PATTERN, newPackage, normalizePackage, reloadThemeStylesheet, slugify } from './lib/package'
import {
  TOKEN_GROUP_ORDER,
  TOKENS,
  setTokenValue,
  type TokenDefinition,
  type TokenGroup,
} from './lib/tokens'
import type { ThemePackage, TokenScope, TokenValue } from './lib/types'
import { THEMES_MANAGE } from './permissions'
import { ThemePreview } from './ThemePreview'
import { TokenField } from './TokenField'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'
import { Alert } from '@/components/ui/alert'

const NEW_THEME_KEY = 'new'

interface EditorLocationState {
  template?: ThemePackage
}

export function ThemeEditorPage() {
  return (
    <RequirePermission permission={THEMES_MANAGE}>
      <ThemeEditorLoader />
    </RequirePermission>
  )
}

function ThemeEditorLoader() {
  const { t } = useTranslation('themes')
  const { key = NEW_THEME_KEY } = useParams()
  const location = useLocation()
  const creating = key === NEW_THEME_KEY
  const template = (location.state as EditorLocationState | null)?.template
  const loaded = useQuery({
    queryKey: themesKeys.package(key),
    queryFn: () => themesApi.exportPackage(key),
    enabled: !creating,
    staleTime: 0,
    gcTime: 0,
  })

  if (!creating && loaded.isPending) return <Skeleton className="h-96 w-full" />
  if (!creating && loaded.isError) {
    return <ErrorState title={t('editor.loadError')} onRetry={() => void loaded.refetch()} />
  }
  return <ThemeEditor key={key} creating={creating} initial={loaded.data ?? template ?? newPackage()} />
}

function groupedTokens(scope: 'color' | 'shared'): [TokenGroup, TokenDefinition[]][] {
  return TOKEN_GROUP_ORDER.map((group): [TokenGroup, TokenDefinition[]] => [
    group,
    TOKENS.filter((token) => token.group === group && (token.kind === 'color') === (scope === 'color')),
  ]).filter(([, tokens]) => tokens.length > 0)
}

function ThemeEditor({ creating, initial }: { creating: boolean; initial: ThemePackage }) {
  const { t } = useTranslation('themes')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const [draft, setDraft] = useState<ThemePackage>(initial)
  const [previewMode, setPreviewMode] = useState<'light' | 'dark'>('light')
  const [fieldIssues, setFieldIssues] = useState<string[]>([])

  const warnings = useMemo(() => contrastWarnings(draft.tokens, previewMode), [draft.tokens, previewMode])
  const idValid = THEME_KEY_PATTERN.test(draft.id)
  const canSave = idValid && draft.name.trim() !== '' && draft.author.trim() !== '' && draft.license.trim() !== ''

  const patch = (changes: Partial<ThemePackage>) => setDraft((current) => ({ ...current, ...changes }))
  const setToken = (scope: TokenScope, name: string, value: TokenValue | undefined) =>
    setDraft((current) => ({ ...current, tokens: setTokenValue(current.tokens, scope, name, value) }))

  const save = useMutation({
    mutationFn: () => themesApi.save(draft.id, normalizePackage(draft)),
    onSuccess: (summary) => {
      if (summary.active) reloadThemeStylesheet()
      void queryClient.invalidateQueries({ queryKey: themesKeys.all })
      notifySuccess(t('editor.saved', { name: summary.name }))
      void navigate('..', { relative: 'path' })
    },
    onError: (error) => {
      const fields = isApiError(error) ? (error.details as { fields?: { field: string; message: string }[] } | null)?.fields : undefined
      setFieldIssues(fields?.map((field) => `${field.field}: ${field.message}`) ?? [])
      notifyError(error)
    },
  })

  const colorGroups = useMemo(() => groupedTokens('color'), [])
  const sharedGroups = useMemo(() => groupedTokens('shared'), [])

  return (
    <Page>
      <PageHeader
        title={creating ? t('editor.createTitle') : t('editor.editTitle', { name: initial.name })}
        description={t('editor.description')}
        actions={
          <div className="flex flex-wrap gap-2">
            <Button variant="outline" size="sm" onClick={() => void navigate('..', { relative: 'path' })}>
              <ArrowLeft aria-hidden="true" />
              {t('actions.back')}
            </Button>
            <Button size="sm" disabled={!canSave || save.isPending} onClick={() => save.mutate()}>
              <Save aria-hidden="true" />
              {save.isPending ? t('editor.saving') : t('editor.save')}
            </Button>
          </div>
        }
      />
      {fieldIssues.length > 0 && (
        <Alert>
          <ul className="grid gap-1">
            {fieldIssues.map((issue) => (
              <li key={issue} className="break-words">
                {issue}
              </li>
            ))}
          </ul>
        </Alert>
      )}
      <div className="grid gap-6 xl:grid-cols-[minmax(0,1fr)_minmax(0,26rem)]">
        <div className="grid min-w-0 content-start gap-6">
          <fieldset className="grid gap-3 sm:grid-cols-2">
            <legend className="mb-2 text-base font-semibold">{t('editor.identity')}</legend>
            <div className="grid gap-1.5">
              <Label htmlFor="theme-name">{t('editor.name')}</Label>
              <Input placeholder={t('placeholders.name')}
                id="theme-name"
                value={draft.name}
                onChange={(event) =>
                  patch({
                    name: event.target.value,
                    ...(creating ? { id: slugify(event.target.value) } : {}),
                  })
                }
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="theme-id">{t('editor.id')}</Label>
              <Input placeholder={t('placeholders.id')}
                id="theme-id"
                value={draft.id}
                disabled={!creating}
                aria-invalid={draft.id !== '' && !idValid}
                onChange={(event) => patch({ id: event.target.value })}
              />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="theme-version">{t('editor.version')}</Label>
              <Input placeholder={t('placeholders.version')} id="theme-version" value={draft.version} onChange={(event) => patch({ version: event.target.value })} />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="theme-author">{t('editor.author')}</Label>
              <Input placeholder={t('placeholders.author')} id="theme-author" value={draft.author} onChange={(event) => patch({ author: event.target.value })} />
            </div>
            <div className="grid gap-1.5">
              <Label htmlFor="theme-license">{t('editor.license')}</Label>
              <Input placeholder={t('placeholders.license')} id="theme-license" value={draft.license} onChange={(event) => patch({ license: event.target.value })} />
            </div>
            <div className="grid gap-1.5 sm:col-span-2">
              <Label htmlFor="theme-description">{t('editor.descriptionLabel')}</Label>
              <Textarea placeholder={t('placeholders.description')}
                id="theme-description"
                rows={2}
                value={draft.description ?? ''}
                onChange={(event) => patch({ description: event.target.value })}
              />
            </div>
          </fieldset>
          <Tabs defaultValue="light" onValueChange={(value) => value !== 'shared' && setPreviewMode(value as 'light' | 'dark')}>
            <TabsList>
              <TabsTrigger value="light">{t('editor.tabs.light')}</TabsTrigger>
              <TabsTrigger value="dark">{t('editor.tabs.dark')}</TabsTrigger>
              <TabsTrigger value="shared">{t('editor.tabs.shared')}</TabsTrigger>
              <TabsTrigger value="fonts">{t('editor.tabs.fonts')}</TabsTrigger>
            </TabsList>
            {(['light', 'dark'] as const).map((mode) => (
              <TabsContent key={mode} value={mode} className="mt-4 grid gap-6">
                {mode === 'dark' && <p className="text-sm text-muted-foreground">{t('editor.darkNotice')}</p>}
                {colorGroups.map(([group, tokens]) => (
                  <TokenGroupFields
                    key={group}
                    group={group}
                    tokens={tokens}
                    mode={mode}
                    values={draft.tokens[mode]}
                    onChange={(name, value) => setToken(mode, name, value)}
                  />
                ))}
              </TabsContent>
            ))}
            <TabsContent value="shared" className="mt-4 grid gap-6">
              {sharedGroups.map(([group, tokens]) => (
                <TokenGroupFields
                  key={group}
                  group={group}
                  tokens={tokens}
                  mode="light"
                  values={draft.tokens.shared}
                  onChange={(name, value) => setToken('shared', name, value)}
                />
              ))}
            </TabsContent>
            <TabsContent value="fonts" className="mt-4">
              <FontsEditor fonts={draft.fonts} onChange={(fonts) => patch({ fonts })} />
            </TabsContent>
          </Tabs>
        </div>
        <aside className="grid min-w-0 content-start gap-3 xl:sticky xl:top-4 xl:self-start" aria-label={t('preview.title')}>
          <div className="flex items-center justify-between gap-2">
            <h3 className="text-base font-semibold">{t('preview.title')}</h3>
            <SegmentedControl
              aria-label={t('preview.mode')}
              size="sm"
              value={previewMode}
              onValueChange={setPreviewMode}
              options={(['light', 'dark'] as const).map((mode) => ({ value: mode, label: t(`editor.tabs.${mode}`) }))}
            />
          </div>
          {warnings.length > 0 && (
            <ul className="grid gap-1 rounded-md border border-warning/40 bg-warning/10 p-3 text-sm">
              {warnings.map((warning) => (
                <li key={`${warning.foreground}:${warning.background}`}>
                  {t('contrast.item', {
                    foreground: warning.foreground,
                    background: warning.background,
                    ratio: warning.ratio.toFixed(1),
                  })}
                </li>
              ))}
            </ul>
          )}
          <ThemePreview tokens={draft.tokens} fonts={draft.fonts} mode={previewMode} />
        </aside>
      </div>
    </Page>
  )
}

interface TokenGroupFieldsProps {
  group: TokenGroup
  tokens: TokenDefinition[]
  mode: 'light' | 'dark'
  values: Record<string, TokenValue>
  onChange: (name: string, value: TokenValue | undefined) => void
}

function TokenGroupFields({ group, tokens, mode, values, onChange }: TokenGroupFieldsProps) {
  const { t } = useTranslation('themes')
  return (
    <fieldset className="grid gap-3 sm:grid-cols-2">
      <legend className="mb-2 text-sm font-semibold">{t(`groups.${group}`)}</legend>
      {tokens.map((token) => (
        <TokenField
          key={token.name}
          token={token}
          mode={mode}
          value={values[token.name]}
          onChange={(value) => onChange(token.name, value)}
        />
      ))}
    </fieldset>
  )
}
