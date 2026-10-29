import { brandPalette, isHexColor } from './contrast'

const ORGANIZATION_PROPERTIES = {
  solid: '--org-primary-solid',
  solidForeground: '--org-primary-foreground',
  lightText: '--org-primary',
  darkText: '--org-primary-dark',
} as const

export function applyOrganizationColor(color: string | null | undefined, root: HTMLElement = document.documentElement) {
  const palette = isHexColor(color) ? brandPalette(color) : null
  for (const [key, property] of Object.entries(ORGANIZATION_PROPERTIES)) {
    if (palette) root.style.setProperty(property, palette[key as keyof typeof ORGANIZATION_PROPERTIES])
    else root.style.removeProperty(property)
  }
}
