import { createElement, useEffect, useRef, type ReactElement } from 'react'
import { useBlocker, type Location } from 'react-router'
import { UnsavedChangesDialog } from './UnsavedChangesDialog'

export type GuardCondition = boolean | (() => boolean)

export interface UnsavedChangesGuard {
  isBlocked: boolean
  proceed: () => void
  stay: () => void
  dialog: ReactElement
}

function evaluate(condition: GuardCondition): boolean {
  return typeof condition === 'function' ? condition() : condition
}

function locationKey(location: Location) {
  return `${location.pathname}${location.search}`
}

function useLatest<T>(value: T) {
  const ref = useRef(value)
  useEffect(() => {
    ref.current = value
  })
  return ref
}

export function useBeforeUnloadGuard(when: GuardCondition) {
  const latest = useLatest(when)
  useEffect(() => {
    const handleBeforeUnload = (event: BeforeUnloadEvent) => {
      if (!evaluate(latest.current)) return
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', handleBeforeUnload)
    return () => window.removeEventListener('beforeunload', handleBeforeUnload)
  }, [latest])
}

export function useUnsavedChangesGuard(when: GuardCondition): UnsavedChangesGuard {
  const latest = useLatest(when)
  useBeforeUnloadGuard(when)
  const blocker = useBlocker(
    ({ currentLocation, nextLocation }) =>
      evaluate(latest.current) && locationKey(currentLocation) !== locationKey(nextLocation),
  )
  const isBlocked = blocker.state === 'blocked'
  const isDirty = evaluate(when)

  useEffect(() => {
    if (isBlocked && !isDirty) blocker.reset()
  }, [blocker, isBlocked, isDirty])

  const proceed = () => {
    if (blocker.state === 'blocked') blocker.proceed()
  }
  const stay = () => {
    if (blocker.state === 'blocked') blocker.reset()
  }

  return {
    isBlocked,
    proceed,
    stay,
    dialog: createElement(UnsavedChangesDialog, { open: isBlocked, onStay: stay, onLeave: proceed }),
  }
}
