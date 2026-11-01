import { useMobileQuickAdd } from '@/features/mobile/quick-add-context'
import { Plus } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { useCommandPalette } from './context'
import { useCreateActions } from './create-actions'

export function CreateMenuButton() {
  const { t } = useTranslation('command-palette')
  const { t: mobile } = useTranslation('mobile')
  const quickAdd = useMobileQuickAdd()
  const { openCreate } = useCommandPalette()
  const actions = useCreateActions()

  if (actions.length === 0 && !quickAdd?.actions.length) return null

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" className={actions.length === 0 ? 'md:hidden' : undefined} aria-label={t('create.menu')}>
          <Plus />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-52">
        <DropdownMenuLabel>{t('create.menu')}</DropdownMenuLabel>
        <DropdownMenuSeparator />
        {actions.map((action) => (
          <DropdownMenuItem key={action.key} onSelect={() => openCreate({ kind: action.key })}>
            <action.icon aria-hidden />
            {action.label}
          </DropdownMenuItem>
        ))}
        {quickAdd && quickAdd.actions.length > 0 && <div className="md:hidden">
          <DropdownMenuSeparator />
          <DropdownMenuLabel>{mobile('quickAdd.menuLabel')}</DropdownMenuLabel>
          {quickAdd.actions.map((action) => <DropdownMenuItem key={action.id} onSelect={() => quickAdd.open(action.id)}><action.icon aria-hidden />{action.label}</DropdownMenuItem>)}
        </div>}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
