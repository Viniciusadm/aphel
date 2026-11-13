import { DocumentEditorPage, DocumentKindContext } from '@/sdk'
import { pageKind } from './api'

export function PageEditorPage() {
  return (
    <DocumentKindContext.Provider value={pageKind}>
      <DocumentEditorPage />
    </DocumentKindContext.Provider>
  )
}
