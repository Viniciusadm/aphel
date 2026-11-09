import { Outlet } from 'react-router'
import { OrganizationTheme } from '@/features/settings/organization/OrganizationTheme'
import { Slot } from '@/kernel/modules/Slot'
import { ExtensionLoadNotice } from './ExtensionLoadNotice'
import { LanguageSync } from './LanguageSync'

export function BareLayout() {
  return (
    <>
      <OrganizationTheme />
      <LanguageSync />
      <ExtensionLoadNotice />
      <Slot name="app.shell" props={{}} />
      <Slot name="app.banner" props={{}} />
      <Outlet />
    </>
  )
}
