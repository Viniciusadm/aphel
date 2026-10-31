import type { ReactNode } from 'react'
import { defaultUiLabels } from './default-labels'
import { UiLabelsContext, type UiLabels } from './labels-context'

export function UiLabelsProvider({ labels, children }: { labels: Partial<UiLabels>; children: ReactNode }) {
  return <UiLabelsContext.Provider value={{ ...defaultUiLabels(), ...labels }}>{children}</UiLabelsContext.Provider>
}
