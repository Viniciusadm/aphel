import type { ComponentProps } from 'react'
import { NativeSelect as UiNativeSelect } from '@/components/ui/native-select'
import { cn } from '@/lib/utils'

export function NativeSelect({ className, ...props }: ComponentProps<'select'>) {
  return <UiNativeSelect className={cn('flex w-full', className)} {...props} />
}
