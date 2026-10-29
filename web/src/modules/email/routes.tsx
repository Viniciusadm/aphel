import type { RouteObject } from 'react-router'

export const mailboxesSettingsRoute: RouteObject = {
  path: 'mailboxes',
  lazy: async () => ({ Component: (await import('./mailboxes/MailboxesPage')).MailboxesPage }),
}

export const emailSettingsRoute: RouteObject = {
  path: 'email',
  lazy: async () => ({ Component: (await import('./settings/EmailSettingsPage')).EmailSettingsPage }),
}
