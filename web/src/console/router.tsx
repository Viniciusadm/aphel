import { createBrowserRouter } from 'react-router'
import { ConsoleLayout } from './components/ConsoleLayout'
import { AuditPage } from './pages/AuditPage'
import { InvitationPage } from './pages/InvitationPage'
import { NewTenantPage } from './pages/NewTenantPage'
import { OperatorsPage } from './pages/OperatorsPage'
import { PasswordPage } from './pages/PasswordPage'
import { SignInPage } from './pages/SignInPage'
import { TenantPage } from './pages/TenantPage'
import { TenantPluginsPage } from './pages/TenantPluginsPage'
import { TenantsPage } from './pages/TenantsPage'
import { AUDIT_PATH, INVITATION_PATH, NEW_TENANT_PATH, OPERATORS_PATH, PASSWORD_PATH, SIGN_IN_PATH } from './paths'

export const consoleRouter = createBrowserRouter([
  { path: SIGN_IN_PATH, element: <SignInPage /> },
  { path: `${INVITATION_PATH}/:token`, element: <InvitationPage /> },
  {
    element: <ConsoleLayout />,
    children: [
      { index: true, element: <TenantsPage /> },
      { path: NEW_TENANT_PATH, element: <NewTenantPage /> },
      { path: '/tenants/:slug', element: <TenantPage /> },
      { path: '/tenants/:slug/plugins', element: <TenantPluginsPage /> },
      { path: OPERATORS_PATH, element: <OperatorsPage /> },
      { path: AUDIT_PATH, element: <AuditPage /> },
      { path: PASSWORD_PATH, element: <PasswordPage /> },
      { path: '*', element: <TenantsPage /> },
    ],
  },
])
