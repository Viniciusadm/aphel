import { createContext, useContext, type ReactNode } from 'react'
import { defaultUiLabels } from './default-labels'

export interface UiLabels {
  sectionNavigation?: string
  close: string
  retry: string
  emptyTitle: string
  errorTitle: string
  errorDescription: string
  toastRegion: string
  toastViewport: string
  confirmCancel: string
  confirmConfirm: string
  confirmDelete: string
  confirmPending: string
  confirmTypeLabel: string
  confirmTypePrompt: (name: string) => ReactNode
  pageLoading: string
  paginationPreviousShort?: string
  paginationNextShort?: string
  displaySettings?: string
  paginationPrevious: string
  paginationNext: string
  paginationSummary: (from: string, to: string, total: string) => string
  paginationPage: (page: number, pages: number) => string
  paginationPerPage: string
  back: string
}

export const UiLabelsContext = createContext<UiLabels | null>(null)

export function useUiLabels(): UiLabels {
  return useContext(UiLabelsContext) ?? defaultUiLabels()
}
