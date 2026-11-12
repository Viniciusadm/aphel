import { useTranslation } from 'react-i18next'
import { Outlet } from 'react-router'
import { Brand } from '@/app/layout/Brand'
import { AuthSplitLayout } from '@/components/auth/AuthSplitLayout'
import { PublicModulesSync } from '@/kernel/modules/ModulesSync'
import { Slot } from '@/kernel/modules/Slot'

export function AuthLayout() {
  const { t } = useTranslation('auth')
  return (
    <AuthSplitLayout
      top={
        <>
          <PublicModulesSync />
          <Slot name="app.banner" props={{}} />
        </>
      }
      brand={<Brand tone="inverted" />}
      tagline={t('layout.tagline')}
      subline={t('layout.subline')}
    >
      <Outlet />
    </AuthSplitLayout>
  )
}
