import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { FileJson, Upload } from 'lucide-react'
import { useId, useRef, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { ListSkeleton } from '@/sdk'
import type {PackageJson} from '@/sdk'
import { readPackageFile, type PackageFileError } from '@/modules/config_packages/ui/lib/package-file'
import {groupByPackage, useInstalledVersions, useInvalidateVersions, versioningApi} from '@/sdk'
import {ComparisonView} from '@/sdk'
import {useDateFormatter} from '@/sdk'
import type {VersionComparison} from '@/sdk'

interface ComparedPackage {
  pkg: PackageJson
  comparison: VersionComparison
}

export function VersionsPanel() {
  const { t } = useTranslation('config-package-versioning')
  const invalidate = useInvalidateVersions()
  const [compared, setCompared] = useState<ComparedPackage | null>(null)
  const [comparing, setComparing] = useState(false)
  const [notice, setNotice] = useState<string | null>(null)

  const compare = async (pkg: PackageJson) => {
    setComparing(true)
    try {
      setCompared({ pkg, comparison: await versioningApi.compare(pkg) })
    } catch (error) {
      notifyError(error)
    } finally {
      setComparing(false)
    }
  }

  const reset = () => {
    setCompared(null)
    setNotice(null)
  }

  if (compared) {
    return (
      <ComparisonView
        comparison={compared.comparison}
        notice={notice}
        refreshing={comparing}
        onBack={reset}
        onApply={(fingerprint, resolutions) => versioningApi.apply(compared.pkg, fingerprint, resolutions)}
        onOutdated={async () => {
          setNotice(t('comparison.outdated'))
          await compare(compared.pkg)
        }}
        onApplied={() => {
          reset()
          void invalidate()
        }}
      />
    )
  }

  return (
    <div className="grid gap-4">
      <UploadCard disabled={comparing} onChoose={(pkg) => void compare(pkg)} />
      <InstalledList />
    </div>
  )
}

function UploadCard({ disabled, onChoose }: { disabled: boolean; onChoose: (pkg: PackageJson) => void }) {
  const { t } = useTranslation('config-package-versioning')
  const inputId = useId()
  const inputRef = useRef<HTMLInputElement>(null)
  const [error, setError] = useState<PackageFileError | null>(null)

  const handleFile = async (file: File | undefined) => {
    if (!file) return
    setError(null)
    const result = await readPackageFile(file)
    if (!result.ok) {
      setError(result.error)
      return
    }
    onChoose(result.package)
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('upload.title')}</CardTitle>
        <CardDescription>{t('upload.description')}</CardDescription>
      </CardHeader>
      <CardContent className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <input
          ref={inputRef}
          id={inputId}
          type="file"
          accept="application/json,.json"
          className="sr-only"
          disabled={disabled}
          onChange={(event) => {
            void handleFile(event.target.files?.[0])
            event.target.value = ''
          }}
        />
        <Button variant="outline" disabled={disabled} onClick={() => inputRef.current?.click()}>
          <Upload aria-hidden="true" />
          {disabled ? t('upload.comparing') : t('upload.choose')}
        </Button>
        {error && (
          <SharedFieldError>
            {t(`upload.errors.${error}`)}
          </SharedFieldError>
        )}
      </CardContent>
    </Card>
  )
}

function InstalledList() {
  const { t } = useTranslation('config-package-versioning')
  const versions = useInstalledVersions()
  const formatter = useDateFormatter()

  if (versions.isPending) return <ListSkeleton rows={2} />
  if (versions.isError) return <ErrorState onRetry={() => void versions.refetch()} />
  const packages = groupByPackage(versions.data)
  if (packages.length === 0) {
    return <EmptyState icon={FileJson} title={t('installed.emptyTitle')} description={t('installed.emptyDescription')} />
  }

  return (
    <section className="grid gap-2" aria-labelledby="installed-versions-title">
      <h3 id="installed-versions-title" className="text-base font-semibold">
        {t('installed.title')}
      </h3>
      <ul className="grid gap-2">
        {packages.map(({ latest, history }) => (
          <li key={latest.package_id} className="grid min-w-0 gap-1 rounded-lg border bg-card p-3 sm:p-4">
            <div className="flex min-w-0 flex-wrap items-center gap-2">
              <span className="min-w-0 font-medium break-words">{latest.name}</span>
              <Badge>{latest.version}</Badge>
              <code className="min-w-0 text-xs break-all text-muted-foreground">{latest.package_id}</code>
            </div>
            <p className="text-xs text-muted-foreground">
              {t('installed.installedAt', {
                date: formatter.format(new Date(latest.installed_at)),
                name: latest.installed_by?.name ?? t('installed.unknownUser'),
              })}
            </p>
            {latest.origin && <p className="text-xs break-all text-muted-foreground">{t('installed.origin', { origin: latest.origin })}</p>}
            {history.length > 0 && (
              <details className="text-xs text-muted-foreground">
                <summary className="cursor-pointer">{t('installed.history', { count: history.length })}</summary>
                <ul className="mt-1 grid gap-0.5">
                  {history.map((version) => (
                    <li key={version.id}>
                      {t('installed.historyItem', {
                        version: version.version,
                        date: formatter.format(new Date(version.installed_at)),
                      })}
                    </li>
                  ))}
                </ul>
              </details>
            )}
          </li>
        ))}
      </ul>
    </section>
  )
}
