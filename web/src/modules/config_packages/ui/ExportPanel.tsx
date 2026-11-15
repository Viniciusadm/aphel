import { useEffect, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { notifyError, notifySuccess } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { ErrorState } from '@/components/ui/error-state'
import { Field } from '@/components/ui/field'
import { Input } from '@/components/ui/input'
import { Skeleton } from '@/components/ui/skeleton'
import { Textarea } from '@/components/ui/textarea'
import { ToggleField } from '@/components/ui/toggle-field'
import { configPackagesApi, SettingsCard, useConfigSections, usePackageLabels, WIDE_FIELD, type ExportBody } from '@/sdk'
import { downloadJson, packageFileName } from '@/modules/config_packages/ui/lib/download'

export const EXPORT_FORM_ID = 'config-package-export'
const MAX_TEXT_LENGTH = 200

const GROUPS = [
  { key: 'data', sections: ['custom_objects', 'fields', 'layouts', 'views', 'tags', 'validation_rules', 'lifecycle'] },
  {
    key: 'sales',
    sections: [
      'workflow',
      'lost_reasons',
      'deal_member_roles',
      'stage_guidance',
      'scoring_models',
      'forecast',
      'pipeline_renewals',
      'call_outcomes',
      'assignment_rules',
    ],
  },
  {
    key: 'catalog',
    sections: ['currencies', 'line_items', 'price_lists', 'discount_limits', 'proposals.templates', 'approval_policies'],
  },
  { key: 'automation', sections: ['task_types', 'email_templates', 'sequences', 'automations'] },
  { key: 'dashboard', sections: ['dashboard'] },
] as const

export interface ExportState {
  exporting: boolean
  canExport: boolean
}

interface ExportPanelProps {
  onStateChange: (state: ExportState) => void
}

function optional(value: string): string | undefined {
  const trimmed = value.trim()
  return trimmed ? trimmed : undefined
}

export function ExportPanel({ onStateChange }: ExportPanelProps) {
  const sections = useConfigSections()

  if (sections.isPending) {
    return (
      <div className="grid gap-4" aria-busy="true">
        <Skeleton className="h-56 w-full" />
        <Skeleton className="h-40 w-full" />
        <Skeleton className="h-40 w-full" />
      </div>
    )
  }
  if (sections.isError) return <ErrorState onRetry={() => void sections.refetch()} />
  return <ExportForm available={sections.data} onStateChange={onStateChange} />
}

function ExportForm({ available, onStateChange }: { available: string[]; onStateChange: (state: ExportState) => void }) {
  const { t } = useTranslation('config-packages')
  const labels = usePackageLabels()
  const [selected, setSelected] = useState<string[]>(available)
  const [name, setName] = useState('')
  const [description, setDescription] = useState('')
  const [author, setAuthor] = useState('')
  const [exporting, setExporting] = useState(false)

  const groups = useMemo(() => {
    const known = new Set<string>(GROUPS.flatMap((group) => group.sections))
    const result: { key: string; sections: string[] }[] = GROUPS.map((group) => ({
      key: group.key,
      sections: group.sections.filter((section) => available.includes(section)),
    }))
    result.push({ key: 'other', sections: available.filter((section) => !known.has(section)) })
    return result.filter((group) => group.sections.length > 0)
  }, [available])

  const canExport = selected.length > 0
  useEffect(() => onStateChange({ exporting, canExport }), [exporting, canExport, onStateChange])

  const setMany = (sections: string[], checked: boolean) =>
    setSelected((current) =>
      available.filter((key) => (sections.includes(key) ? checked : current.includes(key))),
    )

  const submit = async () => {
    if (!canExport) return
    setExporting(true)
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
      notifySuccess(t('export.done', { file: fileName }))
    } catch (error) {
      notifyError(error)
    } finally {
      setExporting(false)
    }
  }

  return (
    <form
      id={EXPORT_FORM_ID}
      className="grid gap-6"
      noValidate
      onSubmit={(event) => {
        event.preventDefault()
        void submit()
      }}
    >
      <div className="flex flex-wrap items-center justify-between gap-2">
        <p className="text-sm text-muted-foreground">{t('export.hint')}</p>
        <Button
          type="button"
          size="sm"
          variant="outline"
          onClick={() => setMany(available, selected.length !== available.length)}
        >
          {t(selected.length === available.length ? 'export.selectNone' : 'export.selectAll')}
        </Button>
      </div>
      {groups.map((group) => {
        const allChecked = group.sections.every((section) => selected.includes(section))
        return (
          <SettingsCard key={group.key} title={t(`export.groups.${group.key as 'data'}`)}>
            {group.sections.map((section) => (
              <ToggleField
                key={section}
                label={labels.section(section)}
                checked={selected.includes(section)}
                onCheckedChange={(checked) => setMany([section], checked)}
              />
            ))}
            {group.sections.length > 1 && (
              <div className={WIDE_FIELD}>
                <Button type="button" size="sm" variant="ghost" onClick={() => setMany([...group.sections], !allChecked)}>
                  {t(allChecked ? 'export.selectNone' : 'export.selectAll')}
                </Button>
              </div>
            )}
          </SettingsCard>
        )
      })}
      {selected.length === 0 && <SharedFieldError>{t('export.noSections')}</SharedFieldError>}
      <SettingsCard title={t('export.identityTitle')} description={t('export.identityDescription')}>
        <Field label={t('export.name')}>
          {(control) => (
            <Input
              {...control}
              value={name}
              maxLength={MAX_TEXT_LENGTH}
              placeholder={t('export.namePlaceholder')}
              onChange={(event) => setName(event.target.value)}
            />
          )}
        </Field>
        <Field label={t('export.author')}>
          {(control) => (
            <Input
              {...control}
              value={author}
              maxLength={MAX_TEXT_LENGTH}
              placeholder={t('export.authorPlaceholder')}
              onChange={(event) => setAuthor(event.target.value)}
            />
          )}
        </Field>
        <Field label={t('export.packageDescription')} className={WIDE_FIELD}>
          {(control) => (
            <Textarea
              {...control}
              value={description}
              rows={3}
              placeholder={t('export.packageDescriptionPlaceholder')}
              onChange={(event) => setDescription(event.target.value)}
            />
          )}
        </Field>
      </SettingsCard>
    </form>
  )
}
