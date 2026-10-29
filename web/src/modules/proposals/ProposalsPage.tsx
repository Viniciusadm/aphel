import { FileText, MoreHorizontal, Plus } from 'lucide-react'
import { useState } from 'react'
import { Link } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { ConfirmDialog, notifyError, notifySuccess } from '@/components/feedback'
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
import {
  MANAGE_PERMISSION,
  PROPOSALS_KEY,
  PROPOSALS_PATH,
  proposalsApi,
  useInvalidateProposals,
  useProposalTemplates,
  type ProposalTemplate,
  type UpdateProposalTemplate,
} from './api'
import { NoAccess } from './NoAccess'

export function ProposalsPage() {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const { hasPermission } = useSession()
  const canManage = hasPermission(MANAGE_PERMISSION)
  const templates = useProposalTemplates(canManage)
  const invalidate = useInvalidateProposals()
  const [deleting, setDeleting] = useState<ProposalTemplate | null>(null)

  if (!canManage) return <NoAccess />

  const update = async (template: ProposalTemplate, changes: UpdateProposalTemplate) => {
    try {
      const updated = await proposalsApi.updateTemplate(template.id, changes)
      notifySuccess(t('list.updated', { name: updated.name }))
      await invalidate()
    } catch (error) {
      notifyError(error)
    }
  }

  const newButton = (
    <Button asChild size="sm">
      <Link to={`${PROPOSALS_PATH}/new`}>
        <Plus aria-hidden="true" />
        {t('list.new')}
      </Link>
    </Button>
  )

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
        <div className="grid gap-1">
          <h1 className="text-xl font-semibold">{t('list.title')}</h1>
          <p className="text-sm text-muted-foreground">{t('list.description')}</p>
        </div>
        {newButton}
      </div>
      {templates.isPending ? (
        <div className="grid gap-2" aria-busy="true">
          <Skeleton className="h-20 w-full" />
          <Skeleton className="h-20 w-full" />
        </div>
      ) : templates.isError ? (
        <ErrorState onRetry={() => void templates.refetch()} />
      ) : templates.data.length === 0 ? (
        <EmptyState
          icon={FileText}
          title={t('list.emptyTitle')}
          description={t('list.emptyDescription')}
          action={newButton}
        />
      ) : (
        <ul className="grid gap-2" aria-label={t('list.listLabel')}>
          {templates.data.map((template) => (
            <TemplateRow
              key={template.id}
              template={template}
              onUpdate={(changes) => update(template, changes)}
              onDelete={() => setDeleting(template)}
            />
          ))}
        </ul>
      )}
      <ConfirmDialog
        open={deleting !== null}
        onOpenChange={(open) => !open && setDeleting(null)}
        title={t('confirmDelete.title')}
        description={deleting ? t('confirmDelete.description', { name: deleting.name }) : ''}
        confirmLabel={t('confirmDelete.action')}
        variant="destructive"
        onConfirm={async () => {
          if (!deleting) return
          await proposalsApi.deleteTemplate(deleting.id)
          notifySuccess(t('list.deleted', { name: deleting.name }))
          await invalidate()
        }}
      />
    </div>
  )
}

interface TemplateRowProps {
  template: ProposalTemplate
  onUpdate: (changes: UpdateProposalTemplate) => void
  onDelete: () => void
}

function TemplateRow({ template, onUpdate, onDelete }: TemplateRowProps) {
  const t = useModuleTranslation(PROPOSALS_KEY)
  const formatting = useRecordFormatting()
  const editPath = `${PROPOSALS_PATH}/${template.id}`

  return (
    <li className="flex flex-col gap-2 rounded-md border bg-card px-3 py-2 sm:flex-row sm:items-start sm:justify-between">
      <div className="grid min-w-0 gap-1">
        <div className="flex flex-wrap items-center gap-2">
          <Link to={editPath} className="font-medium break-words hover:underline">
            {template.name}
          </Link>
          <code className="rounded bg-muted px-1.5 py-0.5 text-xs">{template.key}</code>
          {template.is_default && <Badge variant="secondary">{t('list.default')}</Badge>}
          <Badge variant={template.is_active ? 'success' : 'outline'}>
            {template.is_active ? t('list.active') : t('list.inactive')}
          </Badge>
        </div>
        {template.description && (
          <p className="text-sm break-words text-muted-foreground">{template.description}</p>
        )}
        <p className="text-xs text-muted-foreground">
          {t('list.updatedAt', { date: formatDateTime(template.updated_at, formatting) })}
        </p>
      </div>
      <div className="flex gap-2 self-start">
        <Button asChild size="sm" variant="outline">
          <Link to={editPath}>{t('list.edit')}</Link>
        </Button>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <Button size="icon" variant="ghost" aria-label={t('list.moreActions', { name: template.name })}>
              <MoreHorizontal aria-hidden="true" />
            </Button>
          </DropdownMenuTrigger>
          <DropdownMenuContent align="end">
            {template.is_active && !template.is_default && (
              <DropdownMenuItem onSelect={() => onUpdate({ is_default: true })}>{t('list.makeDefault')}</DropdownMenuItem>
            )}
            <DropdownMenuItem onSelect={() => onUpdate({ is_active: !template.is_active })}>
              {template.is_active ? t('list.deactivate') : t('list.activate')}
            </DropdownMenuItem>
            <DropdownMenuSeparator />
            <DropdownMenuItem className="text-destructive focus:text-destructive" onSelect={onDelete}>
              {t('list.delete')}
            </DropdownMenuItem>
          </DropdownMenuContent>
        </DropdownMenu>
      </div>
    </li>
  )
}
