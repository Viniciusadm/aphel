import { useMutation, useQueryClient } from '@tanstack/react-query'
import { Building2, ChevronDown, History, KeyRound, LogOut, Users } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Link, NavLink, Navigate, Outlet, useLocation, useNavigate } from 'react-router'
import { Button } from '@/components/ui/button'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import { PageLoader } from '@/components/ui/page-loader'
import { cn } from '@/lib/utils'
import { consoleApi } from '../api'
import { notifyConsoleError } from '../errors'
import { AUDIT_PATH, OPERATORS_PATH, PASSWORD_PATH, SIGN_IN_PATH, TENANTS_PATH } from '../paths'
import { useConsoleSession } from '../session'

export function ConsoleLayout() {
  const { t } = useTranslation('console')
  const session = useConsoleSession()
  const location = useLocation()
  const navigate = useNavigate()
  const queryClient = useQueryClient()
  const signOut = useMutation({
    mutationFn: consoleApi.signOut,
    onSuccess: () => {
      queryClient.clear()
      void navigate(SIGN_IN_PATH, { replace: true })
    },
    onError: notifyConsoleError,
  })

  if (session.isPending) return <PageLoader fullScreen />
  if (session.isError) return <Navigate to={SIGN_IN_PATH} replace state={{ from: location.pathname }} />

  const links = [
    { to: TENANTS_PATH, label: t('nav.tenants'), icon: Building2, end: false, active: location.pathname === '/' || location.pathname.startsWith('/tenants') },
    { to: OPERATORS_PATH, label: t('nav.operators'), icon: Users, end: true, active: undefined },
    { to: AUDIT_PATH, label: t('nav.audit'), icon: History, end: true, active: undefined },
  ]

  return (
    <div className="min-h-dvh bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <div className="mx-auto flex h-14 max-w-6xl items-center gap-3 px-4">
          <Link to={TENANTS_PATH} className="shrink-0 font-semibold">
            {t('brand')}
          </Link>
          <nav className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
            {links.map(({ to, label, icon: Icon, end, active }) => (
              <NavLink
                key={to}
                to={to}
                end={end}
                className={({ isActive }) =>
                  cn(
                    'inline-flex shrink-0 items-center gap-2 rounded-md px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground',
                    (active ?? isActive) && 'bg-muted text-foreground',
                  )
                }
              >
                <Icon className="size-4" aria-hidden />
                {label}
              </NavLink>
            ))}
          </nav>
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" size="sm" aria-label={t('nav.menu')}>
                <span className="max-w-40 truncate">{session.data.display_name}</span>
                <ChevronDown className="size-4" aria-hidden />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuItem onSelect={() => void navigate(PASSWORD_PATH)}>
                <KeyRound className="size-4" aria-hidden />
                {t('nav.password')}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem onSelect={() => signOut.mutate()}>
                <LogOut className="size-4" aria-hidden />
                {t('nav.signOut')}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </header>
      <main className="mx-auto max-w-6xl px-4 py-6">
        <Outlet />
      </main>
    </div>
  )
}
