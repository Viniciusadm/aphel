import { useIsModuleActive } from '@/kernel/modules/registry'
import { Building2, Check, ChevronDown, Copy, Pencil, RotateCcw, Save, Star, StarOff, Trash2, Users } from 'lucide-react'
import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { isApiError } from '@/api'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { toast } from '@/components/ui/toast-store'
import { Slot } from '@/kernel/modules/Slot'
import { cn } from '@/lib/utils'
import { type RecordViewConfig, type SavedView } from '@/sdk'
import { ALL_RECORDS_VIEW } from './urlState'
import { ViewFormDialog, type ViewFormValues } from './ViewFormDialog'
import { useViewMutations, viewConfigRequest, type ViewLayoutOptions } from './viewsApi'

export interface ViewSwitcherProps {
  recordType: string
  views: SavedView[]
  activeView?: SavedView
  config: RecordViewConfig
  isModified: boolean
  onSelect: (viewId: string) => void
  onSaved: (view: SavedView) => void
  onDiscard: () => void
  layout?: ViewLayoutOptions
  className?: string
}

type DialogState = { kind: 'create' } | { kind: 'edit'; view: SavedView } | { kind: 'delete'; view: SavedView } | null

function ActiveViewSwitcher({
  recordType,
  views,
  activeView,
  config,
  isModified,
  onSelect,
  onSaved,
  onDiscard,
  layout,
  className,
}: ViewSwitcherProps) {
  const { t } = useTranslation('records-table')
  const { t: tCommon } = useTranslation()
  const mutations = useViewMutations(recordType)
  const [dialog, setDialog] = useState<DialogState>(null)

  const failed = (error: unknown) =>
    toast({
      title: t('views.toast.error'),
      description: isApiError(error) ? error.message : undefined,
      variant: 'destructive',
    })

  const saveChanges = (view: SavedView) =>
    mutations.update.mutate(
      { viewId: view.id, request: viewConfigRequest(config, view, layout) },
      {
        onSuccess: (saved) => {
          toast({ title: t('views.toast.updated'), variant: 'success' })
          onSaved(saved)
        },
        onError: failed,
      },
    )

  const submitForm = (values: ViewFormValues, canManage: boolean) => {
    const base = {
      name: values.name,
      visibility: values.visibility,
      team_id: values.teamId,
      ...(canManage ? { is_default: values.isDefault } : {}),
    }
    if (dialog?.kind === 'edit') {
      mutations.update.mutate(
        { viewId: dialog.view.id, request: base },
        {
          onSuccess: () => {
            toast({ title: t('views.toast.updated'), variant: 'success' })
            setDialog(null)
          },
          onError: failed,
        },
      )
      return
    }
    mutations.create.mutate(
      { ...base, ...viewConfigRequest(config, undefined, layout), view_type: layout?.viewType ?? 'table' },
      {
        onSuccess: (created) => {
          toast({ title: t('views.toast.created'), variant: 'success' })
          setDialog(null)
          onSaved(created)
        },
        onError: failed,
      },
    )
  }

  const toggleUserDefault = (view: SavedView) =>
    mutations.userDefault.mutate(
      { viewId: view.id, enabled: !view.is_user_default },
      {
        onSuccess: () =>
          toast({
            title: view.is_user_default ? t('views.toast.defaultCleared') : t('views.toast.defaultSet'),
            variant: 'success',
          }),
        onError: failed,
      },
    )

  const confirmDelete = (view: SavedView) =>
    mutations.remove.mutate(view.id, {
      onSuccess: () => {
        toast({ title: t('views.toast.deleted'), variant: 'success' })
        setDialog(null)
        if (activeView?.id === view.id) onSelect(ALL_RECORDS_VIEW)
      },
      onError: failed,
    })

  const activeName = activeView?.name ?? t('views.allRecords')

  return (
    <>
      <DropdownMenu modal={false}>
        <DropdownMenuTrigger asChild>
          <Button
            variant="outline"
            size="sm"
            className={cn('max-w-full min-w-0 justify-between', className)}
            aria-label={`${t('views.label')}: ${activeName}`}
          >
            <span className="truncate font-medium">{activeName}</span>
            {isModified && (
              <Badge variant="outline" className="font-normal">
                {t('views.modified')}
              </Badge>
            )}
            <ChevronDown className="text-muted-foreground" aria-hidden />
          </Button>
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start" className="w-72 max-w-[calc(100vw-2rem)]">
          <DropdownMenuLabel>{t('views.list')}</DropdownMenuLabel>
          <DropdownMenuItem onSelect={() => onSelect(ALL_RECORDS_VIEW)}>
            <Check className={cn(activeView && 'invisible')} aria-hidden />
            {t('views.allRecords')}
          </DropdownMenuItem>
          {views.map((view) => (
            <DropdownMenuItem key={view.id} onSelect={() => onSelect(view.id)}>
              <Check className={cn(activeView?.id !== view.id && 'invisible')} aria-hidden />
              <span className="min-w-0 flex-1 truncate">{view.name}</span>
              <ViewBadges view={view} />
            </DropdownMenuItem>
          ))}
          {views.length === 0 && (
            <p className="px-2 py-1.5 text-xs text-muted-foreground">{t('views.none')}</p>
          )}
          <DropdownMenuSeparator />
          {activeView && isModified && activeView.can_edit && (
            <DropdownMenuItem onSelect={() => saveChanges(activeView)}>
              <Save aria-hidden />
              {t('views.saveChanges')}
            </DropdownMenuItem>
          )}
          <DropdownMenuItem onSelect={() => setDialog({ kind: 'create' })}>
            <Copy aria-hidden />
            {t('views.saveAsNew')}
          </DropdownMenuItem>
          {isModified && (
            <DropdownMenuItem onSelect={onDiscard}>
              <RotateCcw aria-hidden />
              {t('views.discardChanges')}
            </DropdownMenuItem>
          )}
          {activeView && (
            <>
              <DropdownMenuSeparator />
              <DropdownMenuLabel className="truncate">{activeView.name}</DropdownMenuLabel>
              <DropdownMenuItem onSelect={() => toggleUserDefault(activeView)}>
                {activeView.is_user_default ? <StarOff aria-hidden /> : <Star aria-hidden />}
                {activeView.is_user_default ? t('views.clearUserDefault') : t('views.setUserDefault')}
              </DropdownMenuItem>
              {activeView.can_edit && (
                <>
                  <DropdownMenuItem onSelect={() => setDialog({ kind: 'edit', view: activeView })}>
                    <Pencil aria-hidden />
                    {t('views.edit')}
                  </DropdownMenuItem>
                  <DropdownMenuItem variant="destructive" onSelect={() => setDialog({ kind: 'delete', view: activeView })}>
                    <Trash2 aria-hidden />
                    {t('views.delete')}
                  </DropdownMenuItem>
                </>
              )}
            </>
          )}
        </DropdownMenuContent>
      </DropdownMenu>
      {activeView && <Slot name="record.list.viewActions" props={{ recordType, viewId: activeView.id }} />}

      <ViewFormDialog
        open={dialog?.kind === 'create' || dialog?.kind === 'edit'}
        mode={dialog?.kind === 'edit' ? 'edit' : 'create'}
        view={dialog?.kind === 'edit' ? dialog.view : undefined}
        pending={mutations.create.isPending || mutations.update.isPending}
        onOpenChange={(open) => !open && setDialog(null)}
        onSubmit={submitForm}
      />

      <Dialog open={dialog?.kind === 'delete'} onOpenChange={(open) => !open && setDialog(null)}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t('views.deleteDialog.title')}</DialogTitle>
            <DialogDescription>
              {t('views.deleteDialog.description', { name: dialog?.kind === 'delete' ? dialog.view.name : '' })}
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialog(null)}>
              {tCommon('actions.cancel')}
            </Button>
            <Button
              variant="destructive"
              disabled={mutations.remove.isPending}
              onClick={() => dialog?.kind === 'delete' && confirmDelete(dialog.view)}
            >
              {t('views.deleteDialog.confirm')}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </>
  )
}

function ViewBadges({ view }: { view: SavedView }) {
  const { t } = useTranslation('records-table')
  return (
    <span className="flex shrink-0 items-center gap-1 text-muted-foreground">
      {view.visibility === 'team' && <Users className="size-3.5" aria-label={t('views.visibility.team')} />}
      {view.visibility === 'organization' && (
        <Building2 className="size-3.5" aria-label={t('views.visibility.organization')} />
      )}
      {view.is_user_default && <Star className="size-3.5 fill-current" aria-label={t('views.userDefault')} />}
      {!view.is_user_default && view.is_default && (
        <Star className="size-3.5" aria-label={t('views.organizationDefault')} />
      )}
    </span>
  )
}

export function ViewSwitcher(props: ViewSwitcherProps) { const active = useIsModuleActive('saved_views'); return active ? <ActiveViewSwitcher {...props} /> : null }
