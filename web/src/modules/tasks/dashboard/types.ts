import type { WidgetRecordRef as RecordRef } from '@/kernel/widgets/types'
interface UserRef { id: string; display_name: string }
export interface PendingActivityItem {
  id: string
  title: string | null
  priority: string | null
  due_at: string | null
  owner: UserRef | null
  record: RecordRef | null
}

interface Bucket<T> { count: number; items: T[] }
export interface TaskPendingData { overdue: Bucket<PendingActivityItem>; due_today: Bucket<PendingActivityItem>; due_tomorrow: Bucket<PendingActivityItem> }
