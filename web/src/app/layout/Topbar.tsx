import { useQueryClient } from '@tanstack/react-query'
import { LogOut, MonitorOff, User, UserRound } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { useNavigate } from 'react-router'
import { useSession } from '@/app/session/session-context'
import { refreshSession } from '@/app/session/session-query'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuRadioGroup,
  DropdownMenuRadioItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { toast } from '@/components/ui/toast-store'
import { CreateMenuButton, PaletteTrigger } from '@/features/command-palette'
import { ACCOUNT_AREA } from '@/kernel/modules/settings'
import { OrganizationBrand } from '@/features/settings/organization/OrganizationBrand'
import { Slot } from '@/kernel/modules/Slot'
import { useUpdatePersonalPreferences } from '@/features/settings/navigation/api'
import type { SupportedLanguage } from '@/features/settings/navigation/types'
import { normalizeLanguage } from './language'

const LANGUAGES: SupportedLanguage[] = ['pt-BR', 'en']

export function UserMenu() {
  const { t, i18n } = useTranslation('layout')
  const { t: tAuth } = useTranslation('auth')
  const { t: tNavigation } = useTranslation('navigation-settings')
  const { me, logout, logoutAll } = useSession()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const updatePreferences = useUpdatePersonalPreferences()
  const currentLanguage = normalizeLanguage(i18n.language) ?? 'pt-BR'

  const changeLanguage = (value: string) => {
    if (value === currentLanguage) return
    updatePreferences.mutate(
      { locale: value as SupportedLanguage },
      {
        onSuccess: () => void refreshSession(queryClient),
        onError: () => toast({ title: tNavigation('saveError'), variant: 'destructive' }),
      },
    )
  }

  const run = (action: () => Promise<void>) => {
    action().catch(() => {
      toast({ title: tAuth('session.logoutError'), variant: 'destructive' })
    })
  }

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" aria-label={t('user.menu')}>
          <User />
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end" className="min-w-56">
        <div className="grid gap-0.5 px-2 py-1.5">
          <span className="truncate text-sm font-medium">{me.user.display_name}</span>
          <span className="truncate text-xs text-muted-foreground">{me.user.email}</span>
        </div>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => navigate(ACCOUNT_AREA.basePath)}>
          <UserRound aria-hidden />
          {t('user.account')}
        </DropdownMenuItem>
        <Slot name="user.menu" props={{}} />
        <DropdownMenuSeparator />
        <DropdownMenuLabel>{tNavigation('language.label')}</DropdownMenuLabel>
        <DropdownMenuRadioGroup value={currentLanguage} onValueChange={changeLanguage}>
          {LANGUAGES.map((language) => (
            <DropdownMenuRadioItem key={language} value={language}>
              {tNavigation(`language.options.${language}`)}
            </DropdownMenuRadioItem>
          ))}
        </DropdownMenuRadioGroup>
        <DropdownMenuSeparator />
        <DropdownMenuItem onSelect={() => run(logout)}>
          <LogOut aria-hidden />
          {tAuth('session.logout')}
        </DropdownMenuItem>
        {logoutAll && (
          <DropdownMenuItem onSelect={() => run(logoutAll)}>
            <MonitorOff aria-hidden />
            {tAuth('session.logoutAll')}
          </DropdownMenuItem>
        )}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}

export function Topbar() {

  return (
    <header className="sticky top-0 z-30 flex h-14 shrink-0 items-center gap-2 border-b bg-background px-4">
      <div className="min-w-0 flex-1 md:hidden"><OrganizationBrand /></div>
      <div role="search" className="hidden min-w-0 max-w-md flex-1 md:block">
        <PaletteTrigger />
      </div>
      <div className="ml-auto flex items-center gap-1">
        <div className="hidden md:block"><CreateMenuButton /></div>
        <Slot name="app.topbar" props={{}} />
        <div className="hidden md:block"><UserMenu /></div>
      </div>
    </header>
  )
}
