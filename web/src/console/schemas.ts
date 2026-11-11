import { z } from 'zod'

export const PASSWORD_MIN = 8
export const PASSWORD_MAX = 128

export const passwordSchema = z.string().min(PASSWORD_MIN, 'validation.passwordLength').max(PASSWORD_MAX, 'validation.passwordLength')
export const emailSchema = z.string().trim().min(1, 'validation.required').email('validation.email')
export const requiredText = z.string().trim().min(1, 'validation.required')
export const slugSchema = z
  .string()
  .trim()
  .regex(/^[a-z0-9](?:[a-z0-9]|-(?=[a-z0-9])){2,39}$/, 'validation.slug')
export const hostSchema = z
  .string()
  .trim()
  .toLowerCase()
  .regex(/^(?=.{1,253}$)[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?(?:\.[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?)*$/, 'validation.host')

export function slugFrom(name: string): string {
  return name
    .normalize('NFD')
    .replace(/[̀-ͯ]/g, '')
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
    .slice(0, 40)
    .replace(/-+$/g, '')
}
