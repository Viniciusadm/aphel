import type { ComponentProps } from 'react'

declare module '@/kernel/modules/contract-components' {
  interface ContractComponentMap {
    'notifications.MentionInput': Omit<ComponentProps<'textarea'>, 'value' | 'ref'> & {
      value: string
      onValueChange?: (value: string) => void
    }
  }
}

export {}
