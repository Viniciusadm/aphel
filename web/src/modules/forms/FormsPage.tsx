import { ClipboardList, MoreHorizontal, Plus } from 'lucide-react'
import { useEffect, useState } from 'react'
import { useLocation, useNavigate } from 'react-router'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
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
import { Skeleton } from '@/components/ui/skeleton'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '@/components/ui/table'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { FORMS_KEY, FORMS_PATH, formsApi, useInvalidateForms, useForms, type LeadForm } from './api'
import { SnippetsDialog } from './SnippetsDialog'
import { SubmissionsDialog } from './SubmissionsDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type PendingConfirmation = { kind: 'delete' | 'rotate'; form: LeadForm }

export function FormsPage() {
  const t = useModuleTranslation(FORMS_KEY)
  const forms = useForms()
  const invalidate = useInvalidateForms()
  const navigate = useNavigate()
  const location = useLocation()
  const [snippetsOf, setSnippetsOf] = useState<LeadForm | null>(null)
  const [embedId, setEmbedId] = useState<string | null>(null)
  const [submissionsOf, setSubmissionsOf] = useState<LeadForm | null>(null)
  const [pending, setPending] = useState<PendingConfirmation | null>(null)

  useEffect(() => {
    const state = location.state as { saved?: string; embed?: string | null } | null
    if (!state?.saved) return
    notifySuccess(t(state.embed ? 'page.created' : 'page.saved', { name: state.saved }))
    if (state.embed) setEmbedId(state.embed)
    void navigate(location.pathname, { replace: true, state: null })
  }, [location, navigate, t])

  useEffect(() => {
    if (!embedId || !forms.data) return
    const created = forms.data.data.find((item) => item.id === embedId)
    if (created) setSnippetsOf(created)
    setEmbedId(null)
  }, [embedId, forms.data])

  const confirm = async ({ kind, form }: PendingConfirmation) => {
    if (kind === 'delete') {
      await formsApi.remove(form.id)
      notifySuccess(t('page.deleted', { name: form.name }))
    } else {
      const rotated = await formsApi.rotatePublicId(form.id)
      notifySuccess(t('page.rotated', { name: form.name }))
      setSnippetsOf(rotated)
    }
    await invalidate()
  }

  return (
    <Page>
<PageHeader title={t('page.title')} description={t('page.description')} actions={<>
        <Button size="sm" onClick={() => void navigate(`${FORMS_PATH}/new`)}>
          <Plus aria-hidden="true" />
          {t('page.new')}
        </Button>
      
      </>} />

      {forms.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : forms.isError ? (
        <ErrorState onRetry={() => void forms.refetch()} />
      ) : forms.data.data.length === 0 ? (
        <EmptyState
          icon={ClipboardList}
          title={t('page.emptyTitle')}
          description={t('page.emptyDescription')}
          action={
            <Button size="sm" onClick={() => void navigate(`${FORMS_PATH}/new`)}>
              <Plus aria-hidden="true" />
              {t('page.new')}
            </Button>
          }
        />
      ) : (
        <Table aria-label={t('page.listLabel')}>
          <TableHeader>
            <TableRow>
              <TableHead scope="col">{t('page.columns.name')}</TableHead>
              <TableHead scope="col">{t('page.columns.status')}</TableHead>
              <TableHead scope="col" className="hidden text-right lg:table-cell">{t('page.columns.fields')}</TableHead>
              <TableHead scope="col" className="hidden text-right md:table-cell">{t('page.columns.accepted')}</TableHead>
              <TableHead scope="col" className="hidden text-right lg:table-cell">{t('page.columns.spam')}</TableHead>
              <TableHead scope="col" className="hidden text-right lg:table-cell">{t('page.columns.failed')}</TableHead>
              <TableHead scope="col" className="hidden md:table-cell">{t('page.columns.last')}</TableHead>
              <TableHead scope="col">
                <span className="sr-only">{t('page.columns.actions')}</span>
              </TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {forms.data.data.map((form) => (
              <FormRow
                key={form.id}
                form={form}
                onEdit={() => void navigate(`${FORMS_PATH}/${form.id}/edit`)}
                onSnippets={() => setSnippetsOf(form)}
                onSubmissions={() => setSubmissionsOf(form)}
                onConfirm={(kind) => setPending({ kind, form })}
              />
            ))}
          </TableBody>
        </Table>
      )}

      <SnippetsDialog form={snippetsOf} onClose={() => setSnippetsOf(null)} />
      <SubmissionsDialog form={submissionsOf} onClose={() => setSubmissionsOf(null)} />
      <ConfirmDialog
        open={pending !== null}
        onOpenChange={(open) => !open && setPending(null)}
        title={pending ? t(`confirm.${pending.kind}.title`) : ''}
        description={pending ? t(`confirm.${pending.kind}.description`, { name: pending.form.name }) : ''}
        confirmLabel={pending ? t(`confirm.${pending.kind}.action`) : undefined}
        variant="destructive"
        onConfirm={() => (pending ? confirm(pending) : undefined)}
      />
    </Page>
  )
}

interface FormRowProps {
  form: LeadForm
  onEdit: () => void
  onSnippets: () => void
  onSubmissions: () => void
  onConfirm: (kind: PendingConfirmation['kind']) => void
}

function FormRow({ form, onEdit, onSnippets, onSubmissions, onConfirm }: FormRowProps) {
  const t = useModuleTranslation(FORMS_KEY)
  const formatting = useRecordFormatting()
  return (
    <TableRow>
      <TableCell className="font-medium break-words">{form.name}</TableCell>
      <TableCell>
        <Badge variant={form.is_active ? 'success' : 'outline'}>
          {form.is_active ? t('page.active') : t('page.inactive')}
        </Badge>
      </TableCell>
      <TableCell className="hidden text-right tabular-nums lg:table-cell">{form.fields?.length ?? 0}</TableCell>
      <TableCell className="hidden text-right tabular-nums md:table-cell">{form.accepted_count}</TableCell>
      <TableCell className="hidden text-right tabular-nums lg:table-cell">{form.spam_count}</TableCell>
      <TableCell className="hidden text-right tabular-nums lg:table-cell">{form.failed_count}</TableCell>
      <TableCell className="hidden whitespace-nowrap md:table-cell">
        {form.last_submission_at ? formatDateTime(form.last_submission_at, formatting) : <span className="text-muted-foreground">{t('page.noSubmissions')}</span>}
      </TableCell>
      <TableCell>
        <div className="flex justify-end gap-2">
          <Button size="sm" variant="outline" onClick={onSubmissions}>
            {t('page.submissions')}
          </Button>
          <Button size="sm" variant="outline" onClick={onEdit}>
            {t('page.edit')}
          </Button>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button size="icon" variant="ghost" aria-label={t('page.moreActions', { name: form.name })}>
                <MoreHorizontal aria-hidden="true" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={onSnippets}>{t('page.embed')}</DropdownMenuItem>
              <DropdownMenuItem onSelect={() => onConfirm('rotate')}>{t('page.rotate')}</DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem variant="destructive" onSelect={() => onConfirm('delete')}>
                {t('page.delete')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </TableCell>
    </TableRow>
  )
}
