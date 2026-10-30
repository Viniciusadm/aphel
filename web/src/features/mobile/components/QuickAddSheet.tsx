import { ArrowLeft } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogTitle } from '@/components/ui/dialog'
import type { ModuleQuickAdd } from '@/kernel/modules/quick-add'
import type { PickedRecord } from './RecordPicker'

export const QUICK_ADD_MENU = 'menu'

export type QuickAddMode = typeof QUICK_ADD_MENU | string

export interface QuickAddSheetProps {
  mode: QuickAddMode | null
  record: PickedRecord | null
  actions: ModuleQuickAdd[]
  onModeChange: (mode: QuickAddMode | null) => void
}

export function QuickAddSheet({ mode, record, actions, onModeChange }: QuickAddSheetProps) {
  const { t } = useTranslation('mobile')
  const close = () => onModeChange(null)
  const current = actions.find((action) => action.id === mode) ?? null
  const Form = current?.render

  return (
    <Dialog open={mode !== null} onOpenChange={(open) => !open && close()}>
      <DialogContent
        className="top-auto bottom-0 left-0 max-h-[92dvh] w-full max-w-full translate-x-0 translate-y-0 rounded-t-2xl rounded-b-none border-x-0 border-b-0 [--dialog-gap:0px] [--dialog-padding:0px] sm:left-1/2 sm:max-w-lg sm:-translate-x-1/2"
      >
        <div className="sticky top-0 z-10 flex items-center gap-2 border-b bg-background px-4 py-3 pr-12">
          {current && (
            <Button
              type="button"
              variant="ghost"
              size="icon"
              className="-ml-2"
              onClick={() => onModeChange(QUICK_ADD_MENU)}
              aria-label={t('quickAdd.back')}
            >
              <ArrowLeft aria-hidden="true" />
            </Button>
          )}
          <div className="grid min-w-0 gap-0.5">
            <DialogTitle className="truncate text-base">{current ? current.title : t('quickAdd.titles.menu')}</DialogTitle>
            <DialogDescription className="truncate text-xs text-muted-foreground">
              {record && current?.linksRecord ? t('quickAdd.linkedTo', { title: record.title }) : t('quickAdd.description')}
            </DialogDescription>
          </div>
        </div>
        <div className="px-4 pt-4">
          {!current && (
            <ul className="grid grid-cols-2 gap-3 pb-[max(1rem,env(safe-area-inset-bottom))]">
              {actions.map(({ id, label, icon: Icon }) => (
                <li key={id}>
                  <button
                    type="button"
                    onClick={() => onModeChange(id)}
                    className="flex min-h-24 w-full flex-col items-center justify-center gap-2 rounded-xl border bg-card p-3 text-sm font-medium hover:bg-accent focus-visible:ring-2 focus-visible:ring-ring focus-visible:outline-none"
                  >
                    <Icon className="size-6 text-primary" aria-hidden="true" />
                    {label}
                  </button>
                </li>
              ))}
            </ul>
          )}
          {Form && <Form key={current.id} onDone={close} initialRecord={current.linksRecord ? record : null} />}
        </div>
      </DialogContent>
    </Dialog>
  )
}
