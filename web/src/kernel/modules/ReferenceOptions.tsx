import type { ReactNode } from 'react'
import type { ReferenceOptionsContext, ReferenceOptionsResult } from './define'
import { useReferenceOptions } from './reference-options'

export interface ReferenceOptionsProps {
  kind: string
  context?: ReferenceOptionsContext
  children: (result: ReferenceOptionsResult) => ReactNode
}

export function ReferenceOptions({ kind, context, children }: ReferenceOptionsProps) {
  return children(useReferenceOptions(kind, context))
}
