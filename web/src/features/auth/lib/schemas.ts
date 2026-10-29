import { z } from 'zod'

export const PASSWORD_MIN_LENGTH = 8
export const PASSWORD_MAX_LENGTH = 128
export const NAME_MAX_LENGTH = 140

export const emailField = z.string().trim().min(1, 'validation.required').pipe(z.email('validation.email'))

export const passwordField = z
  .string()
  .min(PASSWORD_MIN_LENGTH, 'validation.passwordLength')
  .max(PASSWORD_MAX_LENGTH, 'validation.passwordLength')

export const requiredPasswordField = z.string().min(1, 'validation.required')

export const nameField = z.string().trim().min(1, 'validation.required').max(NAME_MAX_LENGTH, 'validation.nameLength')

export const loginSchema = z.object({
  email: emailField,
  password: requiredPasswordField,
})

export const forgotPasswordSchema = z.object({
  email: emailField,
})

export const resetPasswordSchema = z.object({
  password: passwordField,
})

export const newAccountInvitationSchema = z.object({
  displayName: nameField,
  password: passwordField,
})

export const existingAccountInvitationSchema = z.object({
  password: requiredPasswordField,
})

export type LoginValues = z.input<typeof loginSchema>
export type ForgotPasswordValues = z.input<typeof forgotPasswordSchema>
export type ResetPasswordValues = z.input<typeof resetPasswordSchema>
export type NewAccountInvitationValues = z.input<typeof newAccountInvitationSchema>
export type ExistingAccountInvitationValues = z.input<typeof existingAccountInvitationSchema>
