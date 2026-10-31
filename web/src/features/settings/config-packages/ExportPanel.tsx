import { Download } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { notifyError } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { ErrorState } from '@/components/ui/error-state'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { configPackagesApi, useConfigSections, type ExportBody } from './api'
import { downloadJson, packageFileName } from './lib/download'
import { usePackageLabels } from './lib/labels'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

const MAX_TEXT_LENGTH = 200

function optional(value: string): string | undefined {
  const trimmed = value.trim()
  return trimmed ? trimmed : undefined
}

export function ExportPanel() {
  const { t } = useTranslation('config-packages')
  const sections = useConfigSections()

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('export.title')}</CardTitle>
        <CardDescription>{t('export.description')}</CardDescription>
      </CardHeader>
      <CardContent>
        {sections.isPending ? (
          <div className="grid gap-2" aria-busy="true">
            <Skeleton className="h-6 w-full" />
            <Skeleton className="h-6 w-full" />
            <Skeleton className="h-6 w-2/3" />
          </div>
        ) : sections.isError ? (
          <ErrorState onRetry={() => void sections.refetch()} />
        ) : (
          <ExportForm available={sections.data} />
        )}
      </CardContent>
    </Card>
  )
}

function ExportForm({ available }: { available: string[] }) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const [selected, setSelected] = useState<string[]>(available)
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [author, setAuthor] = useState('')
  const [exporting, setExporting] = useState(false)
  const [status, setStatus] = useState('')

  const toggle = (section: string, checked: boolean) =>
    setSelected((current) =>
      checked ? available.filter((key) => key === section || current.includes(key)) : current.filter((key) => key !== section),
    )

  const submit = async () => {
    if (selected.length === 0) return
    setExporting(true)
    setStatus('')
    const body: ExportBody = {
      sections: selected,
      name: optional(name),
      description: optional(description),
      author: optional(author),
    }
    try {
      const exported = await configPackagesApi.exportPackage(body)
      const packageName = typeof exported.name === 'string' && exported.name ? exported.name : (body.name ?? '')
      const fileName = packageFileName(packageName)
      downloadJson(fileName, exported)
      setStatus(t('export.done', { file: fileName }))
    } catch (error) {
      notifyError(error)
    } finally {
      setExporting(false)
    }
  }

  const allSelected = selected.length === available.length

  return (
    <form
      className="grid gap-5"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <fieldset className="grid gap-3">
        <legend className="mb-1 text-sm font-medium">{t('export.sections')}</legend>
        <div className="flex flex-wrap gap-2">
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => setSelected(allSelected ? [] : available)}
          >
            {t(allSelected ? 'export.selectNone' : 'export.selectAll')}
          </Button>
        </div>
        <div className="grid gap-2 sm:grid-cols-2">
          {available.map((section) => {
            return (
              <ToggleField key={section} label={labels.section(section)} checked={selected.includes(section)} onCheckedChange={(checked) => toggle(section, checked === true)} />
            )
          })}
        </div>
        {selected.length === 0 && <p className="text-sm text-destructive">{t('export.noSections')}</p>}
      </fieldset>
      <div className="grid gap-4 sm:grid-cols-2">
        <Field label={t('export.name')}>
          {(control) => <Input
            {...control}
            value={name}
            maxLength={MAX_TEXT_LENGTH}
            placeholder={t('export.namePlaceholder')}
            onChange={(event) => setName(event.target.value)}
          />}
        </Field>
        <Field label={t('export.author')}>
          {(control) => <Input placeholder={t('export.authorPlaceholder')}
            {...control}
            value={author}
            maxLength={MAX_TEXT_LENGTH}
            onChange={(event) => setAuthor(event.target.value)}
          />}
        </Field>
        <Field label={t('export.packageDescription')} className="sm:col-span-2">
          {(control) => <Textarea placeholder={t('export.packageDescriptionPlaceholder')}
            {...control}
            value={description}
            rows={3}
            onChange={(event) => setDescription(event.target.value)}
          />}
        </Field>
      </div>
      <p className="text-xs text-muted-foreground">{t('export.hint')}</p>
      <div className="flex flex-wrap items-center gap-3">
        <Button type="submit" disabled={exporting || selected.length === 0} aria-busy={exporting}>
          <Download aria-hidden="true" />
          {exporting ? t('export.exporting') : t('export.submit')}
        </Button>
        <p className="text-sm text-muted-foreground" role="status" aria-live="polite">
          {status}
        </p>
      </div>
    </form>
  )
}
