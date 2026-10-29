import { z } from 'zod'

export const NAME_MAX_LENGTH = 140
export const ROLE_KEY_MAX_LENGTH = 64

export const nameField = z.string().trim().min(1, 'validation.required').max(NAME_MAX_LENGTH, 'validation.name')

export const inviteMemberSchema = z.object({
  email: z.string().trim().min(1, 'validation.required').pipe(z.email('validation.email')),
  roleId: z.string().min(1, 'validation.role'),
})

export type InviteMemberValues = z.infer<typeof inviteMemberSchema>

export const roleSchema = z.object({
  name: nameField,
  key: z
    .string()
    .trim()
    .max(ROLE_KEY_MAX_LENGTH, 'validation.roleKey')
    .regex(/^[a-z0-9_]*$/, 'validation.roleKey'),
})

export type RoleValues = z.infer<typeof roleSchema>

export const teamSchema = z.object({
  name: nameField,
  parentTeamId: z.string(),
})

export type TeamValues = z.infer<typeof teamSchema>
