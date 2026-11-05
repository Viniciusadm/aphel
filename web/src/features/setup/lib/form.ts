import { z } from 'zod'

export const setupSchema = z.object({
  organizationName: z.string().trim().min(1, 'errors.required').max(140, 'errors.nameLength'),
  timezone: z.string().min(1, 'errors.timezone'),
  currency: z.string().regex(/^[A-Z]{3}$/, 'errors.currency'),
  locale: z.string().min(1, 'errors.locale'),
  adminName: z.string().trim().min(1, 'errors.required').max(140, 'errors.nameLength'),
  adminEmail: z.string().trim().email('errors.email'),
  adminPassword: z.string().min(8, 'errors.passwordLength').max(128, 'errors.passwordLength'),
})

export type SetupValues = z.infer<typeof setupSchema>

export const ORGANIZATION_FIELDS = ['locale', 'organizationName', 'timezone', 'currency'] as const satisfies (keyof SetupValues)[]

export const SERVER_FIELDS: Record<string, keyof SetupValues> = {
  organization_name: 'organizationName',
  timezone: 'timezone',
  currency: 'currency',
  locale: 'locale',
  admin_name: 'adminName',
  admin_email: 'adminEmail',
  admin_password: 'adminPassword',
}

export const SERVER_MESSAGES: Record<keyof SetupValues, string> = {
  organizationName: 'errors.nameLength',
  timezone: 'errors.timezone',
  currency: 'errors.currency',
  locale: 'errors.locale',
  adminName: 'errors.nameLength',
  adminEmail: 'errors.email',
  adminPassword: 'errors.passwordLength',
}
