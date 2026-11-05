import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Check, Copy, Download, EllipsisVertical, Palette, Pencil, Plus, Trash2, Upload } from 'lucide-react'
import { useRef, useState, type ChangeEvent } from 'react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { notifyError, notifySuccess } from '@/components/feedback'
import { ConfirmDialog } from '@/components/feedback/ConfirmDialog'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { ErrorState } from '@/components/ui/error-state'
import { ListSkeleton, RequirePermission } from '@/sdk'
import { themesApi, themesKeys, useThemes } from './api'
import { ImportDialog } from './ImportDialog'
import { downloadPackage, readPackageFile, reloadThemeStylesheet } from './lib/package'
import { emptyTokenSets } from './lib/tokens'
import type { ThemePackage, ThemeSummary, ThemeValidation } from './lib/types'
import { THEMES_MANAGE } from './permissions'
import { ThemeSwatches } from './ThemeSwatches'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

export function ThemesPage() {
  return (
    <RequirePermission permission={THEMES_MANAGE}>
      <ThemesSection />
    </RequirePermission>
  )
}

interface PendingImport {
  themePackage: ThemePackage
  validation: ThemeValidation
}

function ThemesSection() {
  const { t } = useTranslation('themes')
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const themes = useThemes()
  const fileInput = useRef<HTMLInputElement>(null)
  const [pendingImport, setPendingImport] = useState<PendingImport | null>(null)
  const [removing, setRemoving] = useState<ThemeSummary | null>(null)

  const refresh = () => queryClient.invalidateQueries({ queryKey: themesKeys.all })

  const activate = useMutation({
    mutationFn: (key: string | null) => themesApi.activate(key),
    onSuccess: () => {
      reloadThemeStylesheet()
      void refresh()
    },
    onError: (error) => notifyError(error),
  })

  const install = useMutation({
    mutationFn: (themePackage: ThemePackage) => themesApi.install(themePackage),
    onSuccess: (summary) => {
      if (summary.active) reloadThemeStylesheet()
      setPendingImport(null)
      void refresh()
      notifySuccess(t('import.installed', { name: summary.name }))
    },
    onError: (error) => notifyError(error),
  })

  const chooseFile = async (event: ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0]
    event.target.value = ''
    if (!file) return
    try {
      const themePackage = await readPackageFile(file)
      setPendingImport({ themePackage, validation: await themesApi.validate(themePackage) })
    } catch (error) {
      notifyError(error instanceof SyntaxError ? new Error(t('import.invalidFile')) : error)
    }
  }

  const duplicate = async (theme: ThemeSummary) => {
    try {
      const source = await themesApi.exportPackage(theme.key)
      void navigate('new', {
        state: {
          template: {
            ...source,
            id: '',
            name: t('editor.copyName', { name: source.name }),
            tokens: source.tokens ?? emptyTokenSets(),
          },
        },
      })
    } catch (error) {
      notifyError(error)
    }
  }

  const exportTheme = async (theme: ThemeSummary) => {
    try {
      downloadPackage(await themesApi.exportPackage(theme.key))
    } catch (error) {
      notifyError(error)
    }
  }

  return (
    <Page>
      <PageHeader
        title={t('page.title')}
        description={t('page.description')}
        actions={
          <div className="flex flex-wrap gap-2">
            <input
              ref={fileInput}
              type="file"
              accept="application/json,.json"
              className="sr-only"
              aria-label={t('page.import')}
              onChange={(event) => void chooseFile(event)}
              tabIndex={-1}
            />
            <Button variant="outline" size="sm" onClick={() => fileInput.current?.click()}>
              <Upload aria-hidden="true" />
              {t('page.import')}
            </Button>
            <Button size="sm" onClick={() => void navigate('new')}>
              <Plus aria-hidden="true" />
              {t('page.create')}
            </Button>
          </div>
        }
      />
      {themes.isPending ? (
        <ListSkeleton rows={3} />
      ) : themes.isError ? (
        <ErrorState onRetry={() => void themes.refetch()} />
      ) : (
        <ul className="grid gap-3 lg:grid-cols-2">
          <li>
            <article
              className="grid h-full content-between gap-3 rounded-lg border bg-card p-3 sm:p-4"
              aria-label={t('default.name')}
            >
              <div className="grid gap-1">
                <div className="flex flex-wrap items-center gap-2">
                  <Palette className="size-4 text-muted-foreground" aria-hidden="true" />
                  <h3 className="font-medium">{t('default.name')}</h3>
                  {themes.data.active_key === null && <Badge variant="success">{t('card.active')}</Badge>}
                </div>
                <p className="text-sm text-muted-foreground">{t('default.description')}</p>
              </div>
              <div className="flex flex-wrap gap-2">
                <Button
                  size="sm"
                  variant="outline"
                  disabled={themes.data.active_key === null || activate.isPending}
                  onClick={() => activate.mutate(null)}
                >
                  <Check aria-hidden="true" />
                  {t('card.use')}
                </Button>
              </div>
            </article>
          </li>
          {themes.data.themes.map((theme) => (
            <li key={theme.key}>
              <article className="grid h-full content-between gap-3 rounded-lg border bg-card p-3 sm:p-4" aria-label={theme.name}>
                <div className="grid gap-2">
                  <ThemeSwatches tokens={theme.tokens} />
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="min-w-0 font-medium break-words">{theme.name}</h3>
                    <Badge variant="secondary">{theme.version}</Badge>
                    {theme.active && <Badge variant="success">{t('card.active')}</Badge>}
                  </div>
                  {theme.description && <p className="text-sm break-words text-muted-foreground">{theme.description}</p>}
                  <p className="flex flex-wrap gap-x-3 text-xs text-muted-foreground">
                    <span>{t('card.author', { author: theme.author })}</span>
                    <span>{t('card.license', { license: theme.license })}</span>
                    {theme.font_count > 0 && <span>{t('card.fonts', { count: theme.font_count })}</span>}
                    <span>{t(`card.source.${theme.source}`, { defaultValue: theme.source })}</span>
                  </p>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Button
                    size="sm"
                    variant={theme.active ? 'outline' : 'default'}
                    disabled={theme.active || activate.isPending}
                    onClick={() => activate.mutate(theme.key)}
                  >
                    <Check aria-hidden="true" />
                    {t('card.use')}
                  </Button>
                  <DropdownMenu>
                    <DropdownMenuTrigger asChild>
                      <Button size="icon" variant="ghost" aria-label={t('card.more', { name: theme.name })}>
                        <EllipsisVertical aria-hidden="true" />
                      </Button>
                    </DropdownMenuTrigger>
                    <DropdownMenuContent align="start">
                      <DropdownMenuItem onSelect={() => void navigate(encodeURIComponent(theme.key))}>
                        <Pencil aria-hidden="true" />
                        {t('card.edit')}
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => void duplicate(theme)}>
                        <Copy aria-hidden="true" />
                        {t('card.duplicate')}
                      </DropdownMenuItem>
                      <DropdownMenuItem onSelect={() => void exportTheme(theme)}>
                        <Download aria-hidden="true" />
                        {t('card.export')}
                      </DropdownMenuItem>
                      <DropdownMenuSeparator />
                      <DropdownMenuItem variant="destructive" disabled={theme.active} onSelect={() => setRemoving(theme)}>
                        <Trash2 aria-hidden="true" />
                        {t('card.remove')}
                      </DropdownMenuItem>
                    </DropdownMenuContent>
                  </DropdownMenu>
                </div>
              </article>
            </li>
          ))}
        </ul>
      )}
      <ImportDialog
        themePackage={pendingImport?.themePackage ?? null}
        validation={pendingImport?.validation ?? null}
        installing={install.isPending}
        onCancel={() => setPendingImport(null)}
        onInstall={() => pendingImport && install.mutate(pendingImport.themePackage)}
      />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        variant="destructive"
        title={t('remove.title', { name: removing?.name ?? '' })}
        description={t('remove.description')}
        confirmLabel={t('card.remove')}
        onConfirm={async () => {
          if (!removing) return
          await themesApi.remove(removing.key)
          void refresh()
          notifySuccess(t('remove.done', { name: removing.name }))
        }}
      />
    </Page>
  )
}
