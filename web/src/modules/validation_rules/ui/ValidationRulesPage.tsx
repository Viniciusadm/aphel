import { ArrowDown, ArrowUp, ListChecks, MoreHorizontal, Pencil, Plus, Power, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { Link, useNavigate, useParams } from 'react-router'
import { ConfirmDialog, PageLoader } from '@/components/feedback'
import { notifyError, notifySuccess } from '@/components/feedback/notify'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Page } from '@/components/ui/page'
import { PageHeader } from '@/components/ui/page-header'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { RecordTypeSelect, RequirePermission, SETTINGS_MANAGE, useFieldSchema } from '@/sdk'
import { useInvalidateValidationRules, useValidationRules, validationRulesApi } from '@/modules/validation_rules/ui/api'
import { VALIDATION_RULES_PATH } from '@/modules/validation_rules/ui/routes'
import type { ValidationRule } from '@/modules/validation_rules/ui/types'
import { useIsModuleActive } from '@/kernel/modules/registry'

export function ValidationRulesPage() {
  return (
    <RequirePermission permission={SETTINGS_MANAGE}>
      <ValidationRulesSettings />
    </RequirePermission>
  )
}

function ValidationRulesSettings() {
  const { t } = useTranslation('validation-rules')
  const { t: tFields } = useTranslation('fields')
  const quotesActive = useIsModuleActive('quotes')
  const navigate = useNavigate()
  const { recordType = '' } = useParams()
  const rules = useValidationRules(recordType)
  const schema = useFieldSchema(recordType)
  const invalidate = useInvalidateValidationRules()
  const [removing, setRemoving] = useState<ValidationRule | null>(null)
  const [busy, setBusy] = useState(false)
  const fieldLabel = (key: string) => schema.data?.fields.find((field) => field.key === key)?.label ?? key
  const editPath = (id: string) => `${VALIDATION_RULES_PATH}/${recordType}/${id}`

  const run = async (task: () => Promise<unknown>, message: string) => {
    setBusy(true)
    try {
      await task()
      await invalidate()
      notifySuccess(message)
    } catch (error) {
      notifyError(error)
    } finally {
      setBusy(false)
    }
  }

  const toggle = (rule: ValidationRule) =>
    run(
      () => validationRulesApi.update(rule.id, { is_active: !rule.is_active }, rule.updated_at),
      rule.is_active ? t('toasts.deactivated') : t('toasts.activated'),
    )

  const move = (index: number, offset: number) => {
    const list = rules.data ?? []
    const target = index + offset
    if (target < 0 || target >= list.length) return
    const ids = list.map((rule) => rule.id)
    ;[ids[index], ids[target]] = [ids[target], ids[index]]
    void run(() => validationRulesApi.reorder(recordType, ids), t('toasts.reordered'))
  }

  const create = (
    <Button onClick={() => void navigate(editPath('new'))}>
      <Plus aria-hidden="true" />
      {t('new')}
    </Button>
  )

  return (
    <Page>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <>
            <RecordTypeSelect
              value={recordType}
              label={t('recordType')}
              to={(type) => `${VALIDATION_RULES_PATH}/${type}`}
              extraTypes={quotesActive ? [{ value: 'quotes.quote', label: tFields('recordTypes.quote') }] : []}
            />
            {create}
          </>
        }
      />
      {rules.isPending ? (
        <PageLoader />
      ) : rules.isError ? (
        <ErrorState onRetry={() => void rules.refetch()} />
      ) : rules.data.length === 0 ? (
        <EmptyState icon={ListChecks} title={t('empty.title')} description={t('empty.description')} action={create} />
      ) : (
        <Table>
          <caption className="sr-only">{t('list.label')}</caption>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('list.columns.name')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('list.columns.message')}</TableHead>
              <TableHead scope="col" className="hidden sm:table-cell">{t('list.columns.timing')}</TableHead>
              <TableHead scope="col" className="hidden lg:table-cell">{t('list.columns.field')}</TableHead>
              <TableHead scope="col">{t('list.columns.status')}</TableHead>
              <TableHead scope="col" className="text-right">
                <span className="sr-only">{t('list.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {rules.data.map((rule, index) => (
              <TableRow key={rule.id}>
                <TableCell className="max-w-36 sm:max-w-64">
                  <Link to={editPath(rule.id)} className="block truncate font-medium underline-offset-4 hover:underline">
                    {rule.name}
                  </Link>
                </TableCell>
                <TableCell className="hidden max-w-80 truncate text-muted-foreground md:table-cell">{rule.message}</TableCell>
                <TableCell className="hidden sm:table-cell">{t(`timing.${rule.applies_on}`)}</TableCell>
                <TableCell className="hidden text-muted-foreground lg:table-cell">
                  {rule.field ? fieldLabel(rule.field) : t('list.generalError')}
                </TableCell>
                <TableCell>
                  <Badge variant={rule.is_active ? 'success' : 'outline'}>
                    {rule.is_active ? t('list.active') : t('list.inactive')}
                  </Badge>
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-1">
                    <Button asChild size="sm" variant="outline">
                      <Link to={editPath(rule.id)} aria-label={t('list.edit', { name: rule.name })}>
                        <Pencil aria-hidden="true" />
                        <span className="hidden sm:inline">{t('list.editShort')}</span>
                      </Link>
                    </Button>
                    <DropdownMenu>
                      <DropdownMenuTrigger asChild>
                        <Button size="icon" variant="ghost" aria-label={t('list.actions', { name: rule.name })}>
                          <MoreHorizontal aria-hidden="true" />
                        </Button>
                      </DropdownMenuTrigger>
                      <DropdownMenuContent align="end">
                        <DropdownMenuItem disabled={busy} onSelect={() => void toggle(rule)}>
                          <Power aria-hidden="true" />
                          {rule.is_active ? t('list.deactivate') : t('list.activate')}
                        </DropdownMenuItem>
                        <DropdownMenuItem disabled={busy || index === 0} onSelect={() => move(index, -1)}>
                          <ArrowUp aria-hidden="true" />
                          {t('list.moveUpShort')}
                        </DropdownMenuItem>
                        <DropdownMenuItem disabled={busy || index === rules.data.length - 1} onSelect={() => move(index, 1)}>
                          <ArrowDown aria-hidden="true" />
                          {t('list.moveDownShort')}
                        </DropdownMenuItem>
                        <DropdownMenuSeparator />
                        <DropdownMenuItem variant="destructive" onSelect={() => setRemoving(rule)}>
                          <Trash2 aria-hidden="true" />
                          {t('list.removeShort')}
                        </DropdownMenuItem>
                      </DropdownMenuContent>
                    </DropdownMenu>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      )}
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => {
          if (!open) setRemoving(null)
        }}
        title={t('removeDialog.title')}
        description={t('removeDialog.description', { name: removing?.name ?? '' })}
        confirmLabel={t('removeDialog.confirm')}
        variant="destructive"
        pending={busy}
        onConfirm={async () => {
          const rule = removing
          if (!rule) return
          await run(() => validationRulesApi.remove(rule.id), t('toasts.removed'))
          setRemoving(null)
        }}
      />
    </Page>
  )
}
