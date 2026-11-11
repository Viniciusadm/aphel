import { useQuery } from '@tanstack/react-query'

import wasmUrl from './aphel_blocks.wasm?url'

interface Exports {
  memory: WebAssembly.Memory
  blocks_alloc: (length: number) => number
  blocks_free: (pointer: number, length: number) => void
  blocks_validate: (pointer: number, length: number) => bigint
}

export interface ValidationError {
  code: string
  field: string
  message: string
}

interface Answer<T> {
  ok: boolean
  normalized?: T
  error?: ValidationError
}

export type VideoProvider = 'youtube' | 'vimeo'
export type EmbedProvider = 'google_forms' | 'typeform' | 'tally' | 'jotform' | 'calendly'

let instance: Exports | null = null
let loading: Promise<true> | null = null
const encoder = new TextEncoder()
const decoder = new TextDecoder()

async function instantiate(): Promise<WebAssembly.Instance> {
  const response = await fetch(wasmUrl, { credentials: 'same-origin' })
  if (!response.ok) throw new Error(`blocks validation: ${response.status}`)
  if (response.headers.get('content-type')?.startsWith('application/wasm')) {
    return (await WebAssembly.instantiateStreaming(response, {})).instance
  }
  return (await WebAssembly.instantiate(await response.arrayBuffer(), {})).instance
}

export function loadValidation(): Promise<true> {
  loading ??= instantiate()
    .then((loaded) => {
      instance = loaded.exports as unknown as Exports
      return true as const
    })
    .catch((failure: unknown) => {
      loading = null
      throw failure
    })
  return loading
}

function call<T>(kind: string, value: unknown, extra: Record<string, unknown> = {}): Answer<T> {
  if (!instance) throw new Error('blocks validation is not loaded')
  const input = encoder.encode(JSON.stringify({ ...extra, kind, value }))
  const pointer = instance.blocks_alloc(input.length)
  new Uint8Array(instance.memory.buffer, pointer, input.length).set(input)
  const packed = instance.blocks_validate(pointer, input.length)
  instance.blocks_free(pointer, input.length)
  const outputPointer = Number(packed >> 32n)
  const outputLength = Number(packed & 0xffffffffn)
  const answer = JSON.parse(decoder.decode(new Uint8Array(instance.memory.buffer, outputPointer, outputLength))) as Answer<T>
  instance.blocks_free(outputPointer, outputLength)
  return answer
}

export function isSafeLink(link: string): boolean {
  return call('link', link).ok
}

export function videoOf(url: string): { provider: VideoProvider; id: string } | null {
  return call<{ provider: VideoProvider; id: string }>('video', url).normalized ?? null
}

export function embedOf(url: string): { provider: EmbedProvider; id: string } | null {
  return call<{ provider: EmbedProvider; id: string }>('embed', url).normalized ?? null
}

export function isValidEmail(email: string): boolean {
  return call('email', email).ok
}

export function isValidPhone(phone: string): boolean {
  return call('phone', phone).ok
}

export function checkBlock(block: unknown): ValidationError | null {
  return call('block', block).error ?? null
}

export function suggestSlug(title: string, fallback: string): string {
  return call<string>('suggest_slug', title, { fallback }).normalized ?? ''
}

export function slugProblem(slug: string): 'invalid' | 'reserved' | null {
  const answer = call('slug', slug)
  if (answer.ok) return null
  return answer.error?.code === 'slug_reserved' ? 'reserved' : 'invalid'
}

export function useBlocksValidation() {
  return useQuery({ queryKey: ['blocks', 'validation'], queryFn: loadValidation, staleTime: Infinity, gcTime: Infinity, retry: 2 })
}
