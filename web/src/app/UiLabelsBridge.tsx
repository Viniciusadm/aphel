import { useMemo, type ReactNode } from 'react'
import { Trans, useTranslation } from 'react-i18next'
import { UiLabelsProvider } from '@/components/ui/labels'
import type { UiLabels } from '@/components/ui/labels-context'

const TOAST_HOTKEY = 'F8'

export function UiLabelsBridge({ children }: { children: ReactNode }) {
  const { t: common } = useTranslation()
  const { t: detail } = useTranslation('records-detail')

  const labels = useMemo<UiLabels>(
    () => ({
      close: common('actions.close'),
      retry: common('actions.retry'),
      emptyTitle: common('states.emptyTitle'),
      errorTitle: common('states.errorTitle'),
      errorDescription: common('states.errorDescription'),
      toastRegion: common('toast.label'),
      toastViewport: common('toast.viewport', { hotkey: TOAST_HOTKEY }),
      confirmCancel: detail('confirm.cancel'),
      confirmConfirm: detail('confirm.confirm'),
      confirmDelete: detail('confirm.delete'),
      confirmPending: detail('confirm.pending'),
      confirmTypeLabel: detail('confirm.typeToConfirmLabel'),
      confirmTypePrompt: (name) => (
        <Trans
          t={detail}
          i18nKey="confirm.typeToConfirm"
          values={{ name }}
          components={{ strong: <strong className="font-semibold break-all" /> }}
        />
      ),
      pageLoading: detail('pageLoader'),
      paginationPrevious: common('pagination.previous'),
      paginationNext: common('pagination.next'),
      paginationSummary: (from, to, total) => common('pagination.summary', { from, to, total }),
      paginationPage: (page, pages) => common('pagination.page', { page, pages }),
      paginationPerPage: common('pagination.perPage'),
      back: common('actions.back'),
    }),
    [common, detail],
  )

  return <UiLabelsProvider labels={labels}>{children}</UiLabelsProvider>
}
