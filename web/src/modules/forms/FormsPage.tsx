import { ClipboardList, MoreHorizontal, Plus } from 'lucide-react'
import { useState } from 'react'
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
import { formatDateTime, useRecordFormatting } from '@/features/records/formatting'
import { useModuleTranslation } from '@/kernel/modules/i18n'
import { FORMS_KEY, formsApi, useInvalidateForms, useForms, type LeadForm } from './api'
import { FormEditorDialog } from './FormEditorDialog'
import { SnippetsDialog } from './SnippetsDialog'
import { SubmissionsDialog } from './SubmissionsDialog'
import { PageHeader } from '@/components/ui/page-header'
import { Page } from '@/components/ui/page'

type PendingConfirmation = { kind: 'delete' | 'rotate'; form: LeadForm }

export function FormsPage() {
  const t = useModuleTranslation(FORMS_KEY)
  const forms = useForms()
  const invalidate = useInvalidateForms()
  const [editing, setEditing] = useState<LeadForm | null>(null)
  const [creating, setCreating] = useState(false)
  const [snippetsOf, setSnippetsOf] = useState<LeadForm | null>(null)
  const [submissionsOf, setSubmissionsOf] = useState<LeadForm | null>(null)
  const [pending, setPending] = useState<PendingConfirmation | null>(null)

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
        <Button size="sm" onClick={() => setCreating(true)}>
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
            <Button size="sm" onClick={() => setCreating(true)}>
              <Plus aria-hidden="true" />
              {t('page.new')}
            </Button>
          }
        />
      ) : (
        <ul className="grid gap-2" aria-label={t('page.listLabel')}>
          {forms.data.data.map((form) => (
            <FormRow
              key={form.id}
              form={form}
              onEdit={() => setEditing(form)}
              onSnippets={() => setSnippetsOf(form)}
              onSubmissions={() => setSubmissionsOf(form)}
              onConfirm={(kind) => setPending({ kind, form })}
            />
          ))}
        </ul>
      )}

      <FormEditorDialog
        open={creating || editing !== null}
        form={editing}
        onOpenChange={(open) => {
          if (!open) {
            setCreating(false)
            setEditing(null)
          }
        }}
        onSaved={(form, created) => {
          setCreating(false)
          setEditing(null)
          void invalidate()
          notifySuccess(t(created ? 'page.created' : 'page.saved', { name: form.name }))
          if (created) setSnippetsOf(form)
        }}
      />
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
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-start sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <span className="font-medium break-words">{form.name}</span>
          <Badge variant={form.is_active ? 'success' : 'outline'}>
            {form.is_active ? t('page.active') : t('page.inactive')}
          </Badge>
          {form.deal && <Badge variant="secondary">{t('page.createsDeal')}</Badge>}
        </div>
        <p className="text-xs text-muted-foreground">
          {t('page.counters', {
            fields: form.fields?.length ?? 0,
            accepted: form.accepted_count,
            spam: form.spam_count,
            failed: form.failed_count,
          })}
        </p>
        <p className="text-xs text-muted-foreground">
          {form.last_submission_at
            ? t('page.lastSubmission', { date: formatDateTime(form.last_submission_at, formatting) })
            : t('page.noSubmissions')}
        </p>
      </div>
      <div className="flex flex-wrap gap-2 self-start">
        <Button size="sm" variant="outline" onClick={onSubmissions}>
          {t('page.submissions')}
        </Button>
        <Button size="sm" variant="outline" onClick={onSnippets}>
          {t('page.embed')}
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
            <DropdownMenuItem onSelect={() => onConfirm('rotate')}>{t('page.rotate')}</DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem variant="destructive" onSelect={() => onConfirm('delete')}>
              {t('page.delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
