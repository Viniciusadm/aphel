import { useTranslation } from 'react-i18next'
import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { Slot } from '@/kernel/modules/Slot'
import { SettingsNav } from '@/features/settings/components/SettingsNav'
import type { SettingsArea } from '@/kernel/modules/settings'
import { OrganizationBrand } from '@/features/settings/organization/OrganizationBrand'
import { CreateMenuButton } from '@/features/command-palette'
import { UserMenu } from './Topbar'
import { SidebarNav } from './SidebarNav'

interface MobileSidebarProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  settingsArea: SettingsArea | null
  returnPath: string
}

export function MobileSidebar({ open, onOpenChange, settingsArea, returnPath }: MobileSidebarProps) {
  const { t } = useTranslation('layout')
  const close = () => onOpenChange(false)

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent side="left" aria-describedby={undefined} className="bg-sidebar md:hidden">
        <DialogTitle className="sr-only">{t('sidebar.title')}</DialogTitle>
        <div className="flex h-14 items-center px-4">
          <div className="min-w-0 flex-1"><OrganizationBrand /></div>
        </div>
        <div className="flex items-center justify-between gap-2 border-y px-4 py-2"><CreateMenuButton /><UserMenu /></div>
        <div className="flex-1 overflow-y-auto py-2">
          {settingsArea ? (
            <SettingsNav area={settingsArea} returnPath={returnPath} onNavigate={close} />
          ) : (
            <>
              <SidebarNav onNavigate={close} />
              <Slot name="app.sidebar" props={{ onNavigate: close }} />
            </>
          )}
        </div>
      </DialogContent>
    </Dialog>
  )
}
