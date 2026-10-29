import { lazyComponent } from '@/lib/lazy-component'
import type { PhoneFieldInputProps } from './PhoneFieldInput'

export const PhoneFieldInput = lazyComponent<PhoneFieldInputProps>(
  async () => (await import('./PhoneFieldInput')).PhoneFieldInput,
  <div className="h-9 min-w-0 rounded-md border border-input bg-muted/30" />,
)
