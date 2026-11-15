import type { components } from '@/modules/reminders/api-types'

type Schemas = components['schemas']

export type Reminder = Schemas['ReminderResponse']
export type ReminderStatus = Schemas['ReminderStatusSchema']
export type RemindersPage = Schemas['Page_ReminderResponse']
export type CreateReminder = Schemas['CreateReminderRequest']
export type UpdateReminder = Schemas['UpdateReminderRequest']
