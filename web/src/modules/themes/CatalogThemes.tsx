import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Download } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import type { CatalogEntry } from '@/features/settings/catalog/types'
import { themesApi, themesKeys, useThemes } from './api'
import { ImportDialog } from './ImportDialog'
import { reloadThemeStylesheet } from './lib/package'
import type { ThemePackage, ThemeValidation } from './lib/types'

interface PendingInstall {
  themePackage: ThemePackage
  validation: ThemeValidation
}

export function CatalogThemes({ entries }: { entries: CatalogEntry[] }) {
  const { t } = useTranslation('themes')
  const queryClient = useQueryClient()
  const themes = useThemes()
  const [busy, setBusy] = useState<string | null>(null)
  const [pending, setPending] = useState<PendingInstall | null>(null)
  const themeEntries = entries.filter((entry) => entry.kind === 'theme')

  const install = useMutation({
    mutationFn: (themePackage: ThemePackage) => themesApi.install(themePackage),
    onSuccess: (summary) => {
      if (summary.active) reloadThemeStylesheet()
      setPending(null)
      void queryClient.invalidateQueries({ queryKey: themesKeys.all })
      notifySuccess(t('import.installed', { name: summary.name }))
    },
    onError: (error) => notifyError(error),
  })

  const check = async (entry: CatalogEntry) => {
    setBusy(entry.id)
    try {
      const themePackage = await themesApi.downloadCatalogTheme(entry.id)
      setPending({ themePackage, validation: await themesApi.validate(themePackage) })
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(null)
    }
  }

  return (
    <section className="grid gap-2" aria-labelledby="catalog-themes">
      <h3 id="catalog-themes" className="text-base font-semibold">
        {t('catalog.title')}
      </h3>
      <p className="text-sm text-muted-foreground">{t('catalog.description')}</p>
      {themeEntries.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t('catalog.empty')}</p>
      ) : (
        <ul className="grid gap-2 lg:grid-cols-2">
          {themeEntries.map((entry) => {
            const installed = themes.data?.themes.find((theme) => theme.key === entry.id)
            const upToDate = installed?.version === entry.version
            return (
              <li key={entry.id}>
                <article className="grid h-full min-w-0 content-between gap-3 rounded-lg border bg-card p-3 sm:p-4" aria-label={entry.name}>
                  <div className="grid min-w-0 gap-1">
                    <div className="flex flex-wrap items-center gap-2">
                      <h4 className="min-w-0 font-medium break-words">{entry.name}</h4>
                      <Badge variant="secondary">{entry.version}</Badge>
                    </div>
                    {entry.description && <p className="text-sm break-words text-muted-foreground">{entry.description}</p>}
                    <p className="text-xs text-muted-foreground">
                      {entry.author} · {entry.license}
                    </p>
                  </div>
                  <div className="flex flex-wrap items-center gap-2">
                    <Button size="sm" disabled={busy !== null || !entry.compatible || upToDate} onClick={() => void check(entry)}>
                      <Download aria-hidden="true" />
                      {busy === entry.id ? t('catalog.checking') : installed ? t('catalog.update') : t('catalog.install')}
                    </Button>
                    {installed && <span className="text-xs">{t('catalog.installed', { version: installed.version })}</span>}
                  </div>
                </article>
              </li>
            )
          })}
        </ul>
      )}
      <ImportDialog
        themePackage={pending?.themePackage ?? null}
        validation={pending?.validation ?? null}
        installing={install.isPending}
        onCancel={() => setPending(null)}
        onInstall={() => pending && install.mutate(pending.themePackage)}
      />
    </section>
  )
}
