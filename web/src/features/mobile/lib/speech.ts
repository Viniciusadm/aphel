import { useCallback, useEffect, useRef, useState } from 'react'

interface DictationAlternative {
  transcript: string
}

interface DictationResult {
  readonly isFinal: boolean
  readonly length: number
  [index: number]: DictationAlternative
}

interface DictationResultList {
  readonly length: number
  [index: number]: DictationResult
}

interface DictationEvent {
  readonly resultIndex: number
  readonly results: DictationResultList
}

interface DictationErrorEvent {
  readonly error: string
}

interface DictationRecognizer {
  lang: string
  continuous: boolean
  interimResults: boolean
  onresult: ((event: DictationEvent) => void) | null
  onerror: ((event: DictationErrorEvent) => void) | null
  onend: (() => void) | null
  start: () => void
  stop: () => void
  abort: () => void
}

type DictationConstructor = new () => DictationRecognizer

interface DictationWindow {
  SpeechRecognition?: DictationConstructor
  webkitSpeechRecognition?: DictationConstructor
}

export const DICTATION_LANGUAGE = 'pt-BR'

export type DictationFailure = 'denied' | 'no_speech' | 'failed'

function recognizerConstructor(): DictationConstructor | null {
  if (typeof window === 'undefined') return null
  const candidate = window as unknown as DictationWindow
  return candidate.SpeechRecognition ?? candidate.webkitSpeechRecognition ?? null
}

export function isDictationSupported(): boolean {
  return recognizerConstructor() !== null
}

function failureOf(error: string): DictationFailure | null {
  if (error === 'aborted') return null
  if (error === 'not-allowed' || error === 'service-not-allowed') return 'denied'
  if (error === 'no-speech') return 'no_speech'
  return 'failed'
}

export interface DictationOptions {
  onText: (text: string) => void
  onFailure?: (failure: DictationFailure) => void
}

export function useDictation({ onText, onFailure }: DictationOptions) {
  const [listening, setListening] = useState(false)
  const [interim, setInterim] = useState('')
  const recognizerRef = useRef<DictationRecognizer | null>(null)
  const callbacks = useRef({ onText, onFailure })
  const supported = isDictationSupported()

  useEffect(() => {
    callbacks.current = { onText, onFailure }
  }, [onText, onFailure])

  useEffect(() => () => recognizerRef.current?.abort(), [])

  const stop = useCallback(() => {
    recognizerRef.current?.stop()
  }, [])

  const start = useCallback(() => {
    const Recognizer = recognizerConstructor()
    if (!Recognizer || recognizerRef.current) return
    const recognizer = new Recognizer()
    recognizer.lang = DICTATION_LANGUAGE
    recognizer.continuous = true
    recognizer.interimResults = true
    recognizer.onresult = (event) => {
      let pending = ''
      for (let index = event.resultIndex; index < event.results.length; index += 1) {
        const result = event.results[index]
        const text = result[0]?.transcript ?? ''
        if (result.isFinal) {
          const finalText = text.trim()
          if (finalText) callbacks.current.onText(finalText)
        } else {
          pending += text
        }
      }
      setInterim(pending.trim())
    }
    recognizer.onerror = (event) => {
      const failure = failureOf(event.error)
      if (failure) callbacks.current.onFailure?.(failure)
    }
    recognizer.onend = () => {
      recognizerRef.current = null
      setListening(false)
      setInterim('')
    }
    recognizerRef.current = recognizer
    try {
      recognizer.start()
      setListening(true)
    } catch {
      recognizerRef.current = null
      callbacks.current.onFailure?.('failed')
    }
  }, [])

  return { supported, listening, interim, start, stop }
}
