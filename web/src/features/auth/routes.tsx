import type { RouteObject } from 'react-router'
import { AuthLayout } from './components/AuthLayout'
import { GuestOnly } from './components/GuestOnly'
import { AcceptInvitationPage } from './pages/AcceptInvitationPage'
import { ForgotPasswordPage } from './pages/ForgotPasswordPage'
import { LoginPage } from './pages/LoginPage'
import { ResetPasswordPage } from './pages/ResetPasswordPage'
import { VerifyEmailPage } from './pages/VerifyEmailPage'
import { TwoFactorChallengePage } from './two-factor/TwoFactorChallengePage'

export const authRoutes: RouteObject[] = [
  {
    element: <AuthLayout />,
    children: [
      {
        element: <GuestOnly />,
        children: [
          { path: 'login', element: <LoginPage /> },
          { path: 'login/two-factor', element: <TwoFactorChallengePage /> },
        ],
      },
      { path: 'forgot-password', element: <ForgotPasswordPage /> },
      { path: 'reset-password', element: <ResetPasswordPage /> },
      { path: 'verify-email', element: <VerifyEmailPage /> },
      { path: 'accept-invitation', element: <AcceptInvitationPage /> },
    ],
  },
]
