import type { UiLabels } from './labels-context'

const portugueseLabels: UiLabels = {
  close: 'Fechar',
  retry: 'Tentar novamente',
  emptyTitle: 'Nada por aqui ainda',
  errorTitle: 'Algo deu errado',
  errorDescription: 'Não foi possível carregar as informações. Tente novamente.',
  toastRegion: 'Aviso',
  toastViewport: 'Avisos (F8)',
  confirmCancel: 'Cancelar',
  confirmConfirm: 'Confirmar',
  confirmDelete: 'Excluir',
  confirmPending: 'Aguarde…',
  confirmTypeLabel: 'Nome do item',
  confirmTypePrompt: (name) => (
    <>
      Para confirmar, digite <strong className="font-semibold break-all">{name}</strong> abaixo.
    </>
  ),
  pageLoading: 'Carregando página…',
  paginationPrevious: 'Anterior',
  paginationNext: 'Próxima',
  paginationSummary: (page, pages, total) => `Página ${page} de ${pages} · ${total} itens`,
  back: 'Voltar',
}

const englishLabels: UiLabels = {
  close: 'Close',
  retry: 'Try again',
  emptyTitle: 'Nothing here yet',
  errorTitle: 'Something went wrong',
  errorDescription: 'The information could not be loaded. Try again.',
  toastRegion: 'Notice',
  toastViewport: 'Notifications (F8)',
  confirmCancel: 'Cancel',
  confirmConfirm: 'Confirm',
  confirmDelete: 'Delete',
  confirmPending: 'Please wait…',
  confirmTypeLabel: 'Item name',
  confirmTypePrompt: (name) => (
    <>
      To confirm, type <strong className="font-semibold break-all">{name}</strong> below.
    </>
  ),
  pageLoading: 'Loading page…',
  paginationPrevious: 'Previous',
  paginationNext: 'Next',
  paginationSummary: (page, pages, total) => `Page ${page} of ${pages} · ${total} items`,
  back: 'Back',
}

export function defaultUiLabels(): UiLabels {
  const language = typeof document === 'undefined' ? '' : document.documentElement.lang
  return language.toLowerCase().startsWith('en') ? englishLabels : portugueseLabels
}
