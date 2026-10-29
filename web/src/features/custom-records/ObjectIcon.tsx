import type { LucideProps } from 'lucide-react'
import { createElement } from 'react'
import { objectIcon } from './icons'

export interface ObjectIconProps extends LucideProps {
  icon: string | null | undefined
}

export function ObjectIcon({ icon, ...props }: ObjectIconProps) {
  return createElement(objectIcon(icon), props)
}
