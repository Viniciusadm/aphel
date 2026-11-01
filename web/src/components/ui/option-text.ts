import { Children, isValidElement, type ReactNode } from 'react'

export function optionText(node: ReactNode): string {
  return Children.toArray(node).map((child) => {
    if (typeof child === 'string' || typeof child === 'number') return String(child)
    return isValidElement<{ children?: ReactNode }>(child) ? optionText(child.props.children) : ''
  }).join('')
}
