import { Component, type ErrorInfo, type ReactNode } from 'react'
import { useTranslation } from 'react-i18next'
import { ErrorState } from '@/components/ui/error-state'

export interface ErrorFallbackProps {
  error: unknown
  reset: () => void
  className?: string
}

export function ErrorFallback({ reset, className }: ErrorFallbackProps) {
  const { t } = useTranslation('records-detail')
  return (
    <ErrorState
      title={t('errorBoundary.title')}
      description={t('errorBoundary.description')}
      onRetry={reset}
      className={className}
    />
  )
}

export interface ErrorBoundaryProps {
  children: ReactNode
  fallback?: (props: ErrorFallbackProps) => ReactNode
  resetKeys?: unknown[]
  onReset?: () => void
  onError?: (error: unknown, info: ErrorInfo) => void
  className?: string
}

interface ErrorBoundaryState {
  error: unknown
  hasError: boolean
}

function keysChanged(previous: unknown[] = [], next: unknown[] = []) {
  return previous.length !== next.length || previous.some((value, index) => !Object.is(value, next[index]))
}

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = { error: null, hasError: false }

  static getDerivedStateFromError(error: unknown): ErrorBoundaryState {
    return { error, hasError: true }
  }

  componentDidCatch(error: unknown, info: ErrorInfo) {
    this.props.onError?.(error, info)
  }

  componentDidUpdate(previous: ErrorBoundaryProps) {
    if (this.state.hasError && keysChanged(previous.resetKeys, this.props.resetKeys)) this.reset()
  }

  reset = () => {
    this.props.onReset?.()
    this.setState({ error: null, hasError: false })
  }

  render() {
    if (!this.state.hasError) return this.props.children
    const props = { error: this.state.error, reset: this.reset, className: this.props.className }
    return this.props.fallback ? this.props.fallback(props) : <ErrorFallback {...props} />
  }
}
