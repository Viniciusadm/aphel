import { FieldError as SharedFieldError } from '@/components/ui/field-error'
import { CheckCircle2, Play } from 'lucide-react'
import { useMemo, useState } from 'react'
import { useTranslation } from 'react-i18next'
import { errorMessage } from '@/components/feedback'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { NativeSelect } from '@/sdk'
import { Slot } from '@/kernel/modules/Slot'
import {useConfigureImport, useStartImport, useValidateImport, type DuplicateKey, type DuplicateStrategy, type ImportJob, type ImportOptions, type ImportValidation} from '@/sdk'
import { acceptsMultiple, delimiterLabel, duplicateKeysFor, missingRequired, supportsReferenceCreation } from '../mapping'
import { ValidationReport } from './ValidationReport'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { Field } from '@/components/ui/field'
import { ToggleField } from '@/components/ui/toggle-field'

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
          <Field label={t('mapping.duplicateKey')}>
            {(control) => <NativeSelect
              {...control}
              value={options.duplicate_key}
              onChange={(event) => changeOptions({ duplicate_key: event.target.value as DuplicateKey })}
            >
              {duplicateKeysFor(job).map((key) => (
                <option key={key} value={key}>
                  {t(`mapping.keys.${key}`)}
                </option>
              ))}
            </NativeSelect>}
          </Field>
          <Field label={t('mapping.duplicateStrategy')}>
            {(control) => <NativeSelect
              {...control}
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
            </NativeSelect>}
          </Field>
          {supportsReferenceCreation(job.targets) && (
            <ToggleField label={t('mapping.createReferences')} checked={options.create_missing_references} onCheckedChange={(checked) => changeOptions({ create_missing_references: checked === true })} className="sm:col-span-2" />
          )}
          <div className="sm:col-span-2">
            <Slot
              name="assignment.ruleSelect"
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
        <SharedFieldError>
          {t('mapping.missingRequired', { fields: missing.map((target) => target.label).join(', ') })}
        </SharedFieldError>
      )}
      {failure && (
        <SharedFieldError>
          {errorMessage(failure)}
        </SharedFieldError>
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
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead scope="col" className="whitespace-nowrap">
                {t('mapping.line')}
              </TableHead>
              {job.headers.map((header, index) => (
                <TableHead key={`${header}-${index}`} scope="col" className="whitespace-nowrap">
                  {header}
                </TableHead>
              ))}
            </TableRow>
          </TableHeader>
          <TableBody>
            {job.preview.map((row) => (
              <TableRow key={row.row_index}>
                <TableCell className="text-muted-foreground tabular-nums">{row.row_index}</TableCell>
                {job.headers.map((header, index) => (
                  <TableCell key={`${header}-${index}`} className="max-w-48 truncate whitespace-nowrap">
                    {row.cells[index] ?? ''}
                  </TableCell>
                ))}
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  )
}
