import { Outlet, useLocation } from 'react-router'
import { Brand } from '@/app/layout/Brand'

export function AuthLayout() {
  const { pathname } = useLocation()
  return (
    <div className="flex min-h-screen flex-col bg-muted/40 text-foreground">
      <header className="flex h-14 items-center px-4">
        <Brand />
      </header>
      <main className="flex flex-1 items-start justify-center px-4 py-8 sm:items-center">
        <div className={pathname === "/install" ? "w-full max-w-3xl" : "w-full max-w-sm"}>
          <Outlet />
        </div>
      </main>
    </div>
  )
}
