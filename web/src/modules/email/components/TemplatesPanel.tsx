import { FileText, Pencil, Plus, Trash2 } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { ConfirmDialog, notifySuccess } from '@/components/feedback'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { EmptyState } from '@/components/ui/empty-state'
import { ErrorState } from '@/components/ui/error-state'
import { Skeleton } from '@/components/ui/skeleton'
import { formatDateTime, useRecordFormatting } from '@/sdk'
import {useEmailTemplates, useRemoveTemplate} from '@/sdk'
import {useRecordTypeLabel} from '@/sdk'
import type {EmailTemplate, TemplateScope} from '@/sdk'
import { TemplateEditorDialog } from './TemplateEditorDialog'

export function TemplatesPanel({ scope }: { scope: TemplateScope }) {
  const { t } = useTranslation('email')
  const formatting = useRecordFormatting()
  const templates = useEmailTemplates(scope)
  const remove = useRemoveTemplate()
  const recordTypeLabel = useRecordTypeLabel()
  const [editing, setEditing] = useState<EmailTemplate | null>(null)
  const [editorOpen, setEditorOpen] = useState(false)
  const [removing, setRemoving] = useState<EmailTemplate | null>(null)

  const openEditor = (template: EmailTemplate | null) => {
    setEditing(template)
    setEditorOpen(true)
  }

  return (
    <div className="grid gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <p className="text-sm text-muted-foreground">{t(`templates.scopeDescription.${scope}`)}</p>
        <Button onClick={() => openEditor(null)} className="w-full sm:w-auto">
          <Plus aria-hidden="true" />
          {t('templates.create')}
        </Button>
      </div>
      {templates.isPending && (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-16 w-full" />
          <Skeleton className="h-16 w-full" />
        </div>
      )}
      {templates.isError && <ErrorState onRetry={() => void templates.refetch()} />}
      {templates.isSuccess && templates.data.length === 0 && (
        <EmptyState icon={FileText} title={t('templates.emptyTitle')} description={t('templates.emptyDescription')} />
      )}
      {templates.isSuccess && templates.data.length > 0 && (
        <ul className="grid gap-2">
          {templates.data.map((template) => (
            <li key={template.id} className="flex flex-col gap-2 rounded-lg border p-3 sm:flex-row sm:items-center">
              <div className="grid min-w-0 flex-1 gap-1">
                <div className="flex flex-wrap items-center gap-2">
                  <span className="font-medium break-words">{template.name}</span>
                  <Badge variant="outline">
                    {template.record_type ? recordTypeLabel(template.record_type) : t('templates.anyRecordType')}
                  </Badge>
                </div>
                <p className="truncate text-sm text-muted-foreground">{template.subject}</p>
                <p className="text-xs text-muted-foreground">
                  {t('templates.updatedAt', { date: formatDateTime(template.updated_at, formatting) })}
                </p>
              </div>
              <div className="flex gap-1 self-end sm:self-auto">
                <Button variant="ghost" size="icon" aria-label={t('actions.edit')} onClick={() => openEditor(template)}>
                  <Pencil />
                </Button>
                <Button variant="ghost" size="icon" aria-label={t('actions.remove')} onClick={() => setRemoving(template)}>
                  <Trash2 />
                </Button>
              </div>
            </li>
          ))}
        </ul>
      )}
      <TemplateEditorDialog open={editorOpen} onOpenChange={setEditorOpen} scope={scope} template={editing} />
      <ConfirmDialog
        open={removing !== null}
        onOpenChange={(open) => !open && setRemoving(null)}
        variant="destructive"
        title={t('templates.removeTitle', { name: removing?.name ?? '' })}
        description={t('templates.removeDescription')}
        onConfirm={async () => {
          if (!removing) return
          await remove.mutateAsync(removing.id)
          notifySuccess(t('templates.removed'))
        }}
      />
    </div>
  )
}
