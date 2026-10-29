import { CheckCircle2, Play } from 'lucide-react'
import { useId, useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Checkbox } from '@/components/ui/checkbox'
import { Label } from '@/components/ui/label'
import { NativeSelect } from '@/features/records/filters/NativeSelect'
import { Slot } from '@/kernel/modules/Slot'
import {
  useConfigureImport,
  useStartImport,
  useValidateImport,
  type DuplicateKey,
  type DuplicateStrategy,
  type ImportJob,
  type ImportOptions,
  type ImportValidation,
} from '../api'
import { acceptsMultiple, delimiterLabel, duplicateKeysFor, missingRequired, supportsCompanyCreation } from '../mapping'
import { ValidationReport } from './ValidationReport'

const STRATEGIES: DuplicateStrategy[] = ['skip', 'update', 'create']

export interface MappingStepProps {
  job: ImportJob
}

export function MappingStep({ job }: MappingStepProps) {
  const { t } = useTranslation('imports')
  const [mapping, setMapping] = useState<(string | null)[]>(() => job.mapping.map((key) => key ?? null))
  const [options, setOptions] = useState<ImportOptions>(job.options)
  const [report, setReport] = useState<ImportValidation | null>(null)
  const configure = useConfigureImport(job.id)
  const validate = useValidateImport(job.id)
  const start = useStartImport(job.id)
  const keyId = useId()
  const strategyId = useId()
  const companiesId = useId()

  const missing = useMemo(
    () => missingRequired(job.targets, mapping),
    [job.targets, mapping],
  )
  const busy = configure.isPending || validate.isPending || start.isPending
  const failure = configure.error ?? validate.error ?? start.error
  const sample = job.preview[0]?.cells ?? []
  const used = new Set(mapping.filter(Boolean))

  const changeColumn = (index: number, key: string) => {
    setReport(null)
    setMapping((current) => current.map((value, position) => (position === index ? key || null : value)))
  }

  const changeOptions = (patch: Partial<ImportOptions>) => {
    setReport(null)
    setOptions((current) => ({ ...current, ...patch }))
  }

  const save = () => configure.mutateAsync({ mapping, options })

  const runValidation = async () => {
    try {
      await save()
      setReport(await validate.mutateAsync())
    } catch {
      setReport(null)
    }
  }

  const runImport = async () => {
    try {
      await save()
      await start.mutateAsync()
    } catch {
      return
    }
  }

  return (
    <div className="grid min-w-0 gap-4">
      <Card>
        <CardHeader>
          <CardTitle>{t('mapping.title')}</CardTitle>
          <CardDescription>
            {t('mapping.description')}{' '}
            {t('mapping.detected', {
              rows: job.row_count,
              encoding: job.encoding.toUpperCase(),
              delimiter: delimiterLabel(job.delimiter),
            })}
          </CardDescription>
        </CardHeader>
        <CardContent className="grid gap-3">
          <div className="hidden grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.2fr)] gap-3 px-1 text-xs font-medium text-muted-foreground sm:grid">
            <span>{t('mapping.column')}</span>
            <span>{t('mapping.sample')}</span>
            <span>{t('mapping.field')}</span>
          </div>
          <ul className="grid gap-2">
            {job.headers.map((header, index) => (
              <li
                key={`${header}-${index}`}
                className="grid min-w-0 gap-2 rounded-md border p-3 sm:grid-cols-[minmax(0,1fr)_minmax(0,1fr)_minmax(0,1.2fr)] sm:items-center sm:gap-3 sm:border-0 sm:p-1"
              >
                <span className="truncate font-medium">{header}</span>
                <span className="truncate text-sm text-muted-foreground">{sample[index] ?? ''}</span>
                <NativeSelect
                  className="w-full"
                  aria-label={t('mapping.fieldFor', { column: header })}
                  value={mapping[index] ?? ''}
                  onChange={(event) => changeColumn(index, event.target.value)}
                >
                  <option value="">{t('mapping.ignore')}</option>
                  {job.targets.map((target) => (
                    <option
                      key={target.key}
                      value={target.key}
                      disabled={used.has(target.key) && mapping[index] !== target.key}
                    >
                      {target.label}
                      {target.required ? ` (${t('mapping.required')})` : ''}
                      {target.is_custom ? ` · ${t('mapping.custom')}` : ''}
                      {acceptsMultiple(target) ? ` · ${t('mapping.multiple')}` : ''}
                    </option>
                  ))}
                </NativeSelect>
              </li>
            ))}
          </ul>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle>{t('mapping.options')}</CardTitle>
        </CardHeader>
        <CardContent className="grid gap-4 sm:grid-cols-2">
          <div className="grid gap-2">
            <Label htmlFor={keyId}>{t('mapping.duplicateKey')}</Label>
            <NativeSelect
              id={keyId}
              value={options.duplicate_key}
              onChange={(event) => changeOptions({ duplicate_key: event.target.value as DuplicateKey })}
            >
              {duplicateKeysFor(job).map((key) => (
                <option key={key} value={key}>
                  {t(`mapping.keys.${key}`)}
                </option>
              ))}
            </NativeSelect>
          </div>
          <div className="grid gap-2">
            <Label htmlFor={strategyId}>{t('mapping.duplicateStrategy')}</Label>
            <NativeSelect
              id={strategyId}
              value={options.duplicate_strategy}
              disabled={options.duplicate_key === 'none'}
              onChange={(event) =>
                changeOptions({ duplicate_strategy: event.target.value as DuplicateStrategy })
              }
            >
              {STRATEGIES.map((strategy) => (
                <option key={strategy} value={strategy}>
                  {t(`mapping.strategies.${strategy}`)}
                </option>
              ))}
            </NativeSelect>
          </div>
          {supportsCompanyCreation(job.record_type, job.targets) && (
            <div className="flex items-center gap-2 sm:col-span-2">
              <Checkbox
                id={companiesId}
                checked={options.create_missing_companies}
                onCheckedChange={(checked) => changeOptions({ create_missing_companies: checked === true })}
              />
              <Label htmlFor={companiesId}>{t('mapping.createCompanies')}</Label>
            </div>
          )}
          <div className="sm:col-span-2">
            <Slot
              name="leadDistribution.ruleSelect"
              props={{
                recordType: job.record_type,
                value: options.distribution_rule_id ?? null,
                onChange: (ruleId) => changeOptions({ distribution_rule_id: ruleId }),
              }}
            />
          </div>
        </CardContent>
      </Card>

      <PreviewTable job={job} />

      {report && <ValidationReport report={report} />}

      {missing.length > 0 && (
        <p role="alert" className="text-sm text-destructive">
          {t('mapping.missingRequired', { fields: missing.map((target) => target.label).join(', ') })}
        </p>
      )}
      {failure && (
        <p role="alert" className="text-sm text-destructive">
          {errorMessage(failure)}
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" disabled={busy} onClick={() => void runValidation()}>
          <CheckCircle2 aria-hidden />
          {validate.isPending ? t('mapping.validating') : t('mapping.validate')}
        </Button>
        <Button disabled={busy || missing.length > 0} onClick={() => void runImport()}>
          <Play aria-hidden />
          {start.isPending ? t('mapping.starting') : t('mapping.start', { count: job.row_count })}
        </Button>
      </div>
    </div>
  )
}

function PreviewTable({ job }: { job: ImportJob }) {
  const { t } = useTranslation('imports')
  if (job.preview.length === 0) return null
  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('mapping.previewTitle')}</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="relative max-w-full overflow-x-auto rounded-md border">
          <table className="w-full text-sm">
            <thead className="bg-muted/50 text-left">
              <tr>
                <th scope="col" className="px-3 py-2 font-medium whitespace-nowrap">
                  {t('mapping.line')}
                </th>
                {job.headers.map((header, index) => (
                  <th key={`${header}-${index}`} scope="col" className="px-3 py-2 font-medium whitespace-nowrap">
                    {header}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {job.preview.map((row) => (
                <tr key={row.row_index} className="border-t">
                  <td className="px-3 py-2 text-muted-foreground tabular-nums">{row.row_index}</td>
                  {job.headers.map((header, index) => (
                    <td key={`${header}-${index}`} className="max-w-48 truncate px-3 py-2 whitespace-nowrap">
                      {row.cells[index] ?? ''}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  )
}
