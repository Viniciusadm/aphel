export const MODIFIERS = ['mod', 'ctrl', 'alt', 'shift', 'meta'] as const
export type Modifier = (typeof MODIFIERS)[number]

export const MAX_SEQUENCE_STEPS = 2
export const MAX_CHORDS_PER_ACTION = 4

const NAMED_KEYS = new Set([
  'enter',
  'escape',
  'space',
  'tab',
  'backspace',
  'delete',
  'insert',
  'home',
  'end',
  'pageup',
  'pagedown',
  'arrowup',
  'arrowdown',
  'arrowleft',
  'arrowright',
  'plus',
])

const MODIFIER_KEYS = new Set(['control', 'shift', 'alt', 'meta', 'altgraph', 'capslock', 'os', 'fn', 'dead', 'unidentified', 'process'])

const KEY_ALIASES: Record<string, string> = {
  ' ': 'space',
  '+': 'plus',
  esc: 'escape',
  del: 'delete',
  up: 'arrowup',
  down: 'arrowdown',
  left: 'arrowleft',
  right: 'arrowright',
  return: 'enter',
}

export function isApplePlatform(): boolean {
  return typeof navigator !== 'undefined' && /mac|iphone|ipad/i.test(navigator.platform || navigator.userAgent)
}

export function isFunctionKey(key: string): boolean {
  const match = /^f([1-9]|1\d|2[0-4])$/.exec(key)
  return match !== null
}

function normalizeKey(raw: string): string | null {
  const lowered = raw.toLowerCase()
  const key = KEY_ALIASES[lowered] ?? KEY_ALIASES[raw] ?? lowered
  if (NAMED_KEYS.has(key) || isFunctionKey(key)) return key
  const chars = [...key]
  if (chars.length !== 1) return null
  const [single] = chars
  if (single === '+' || /\s/.test(single) || single.charCodeAt(0) < 32) return null
  return single
}

export function normalizeStroke(stroke: string): string | null {
  const tokens = stroke.split('+')
  const rawKey = tokens.pop()
  if (rawKey === undefined) return null
  const present = new Set<Modifier>()
  for (const token of tokens) {
    const modifier = token.toLowerCase() as Modifier
    if (!MODIFIERS.includes(modifier) || present.has(modifier)) return null
    present.add(modifier)
  }
  const key = normalizeKey(rawKey)
  if (key === null) return null
  return [...MODIFIERS.filter((modifier) => present.has(modifier)), key].join('+')
}

export function normalizeChord(chord: string): string | null {
  const steps = chord.trim().split(' ')
  if (steps.length === 0 || steps.length > MAX_SEQUENCE_STEPS) return null
  const normalized = steps.map(normalizeStroke)
  if (normalized.some((step) => step === null)) return null
  return normalized.join(' ')
}

export function chordSteps(chord: string): string[] {
  return chord.split(' ')
}

export function strokeParts(stroke: string): { modifiers: Modifier[]; key: string } {
  const tokens = stroke.split('+')
  const key = tokens.pop() ?? ''
  return { modifiers: tokens as Modifier[], key }
}

export function canonicalStroke(stroke: string, apple = isApplePlatform()): string {
  const { modifiers, key } = strokeParts(stroke)
  const mapped = new Set<Modifier>(
    modifiers.map((modifier) => {
      if (!apple && modifier === 'ctrl') return 'mod'
      if (apple && modifier === 'meta') return 'mod'
      return modifier
    }),
  )
  return [...MODIFIERS.filter((modifier) => mapped.has(modifier)), key].join('+')
}

export function canonicalChord(chord: string, apple = isApplePlatform()): string {
  return chordSteps(chord)
    .map((step) => canonicalStroke(step, apple))
    .join(' ')
}

function keyFromEvent(event: KeyboardEvent, useCode: boolean): string | null {
  if (useCode) {
    const letter = /^Key([A-Z])$/.exec(event.code)
    if (letter) return letter[1].toLowerCase()
    const digit = /^(?:Digit|Numpad)(\d)$/.exec(event.code)
    if (digit) return digit[1]
  }
  return normalizeKey(event.key)
}

export function strokeFromEvent(event: KeyboardEvent, apple = isApplePlatform()): string | null {
  if (MODIFIER_KEYS.has(event.key.toLowerCase())) return null
  const commandLike = event.altKey || event.ctrlKey || event.metaKey
  const key = keyFromEvent(event, commandLike)
  if (key === null) return null
  const printable = key.length === 1 && !/[a-z0-9]/.test(key)
  const modifiers = new Set<Modifier>()
  if (apple ? event.metaKey : event.ctrlKey) modifiers.add('mod')
  if (apple && event.ctrlKey) modifiers.add('ctrl')
  if (!apple && event.metaKey) modifiers.add('meta')
  if (event.altKey) modifiers.add('alt')
  if (event.shiftKey && !(printable && !commandLike) && key !== 'plus') modifiers.add('shift')
  return [...MODIFIERS.filter((modifier) => modifiers.has(modifier)), key].join('+')
}

export function strokeHasCommandModifier(stroke: string): boolean {
  const { modifiers } = strokeParts(stroke)
  return modifiers.some((modifier) => modifier !== 'shift')
}

export function strokeIsFunctionKey(stroke: string): boolean {
  return isFunctionKey(strokeParts(stroke).key)
}

const KEY_LABELS: Record<string, string> = {
  enter: 'Enter',
  escape: 'Esc',
  space: 'Space',
  tab: 'Tab',
  backspace: '⌫',
  delete: 'Del',
  insert: 'Ins',
  home: 'Home',
  end: 'End',
  pageup: 'PgUp',
  pagedown: 'PgDn',
  arrowup: '↑',
  arrowdown: '↓',
  arrowleft: '←',
  arrowright: '→',
  plus: '+',
}

const APPLE_MODIFIER_LABELS: Record<Modifier, string> = { mod: '⌘', ctrl: '⌃', alt: '⌥', shift: '⇧', meta: '⌘' }
const OTHER_MODIFIER_LABELS: Record<Modifier, string> = { mod: 'Ctrl', ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift', meta: 'Win' }

export function keyLabel(key: string): string {
  if (KEY_LABELS[key]) return KEY_LABELS[key]
  if (isFunctionKey(key)) return key.toUpperCase()
  return key.toUpperCase()
}

export function formatStroke(stroke: string, apple = isApplePlatform()): string[] {
  const { modifiers, key } = strokeParts(canonicalStroke(stroke, apple))
  const labels = apple ? APPLE_MODIFIER_LABELS : OTHER_MODIFIER_LABELS
  return [...modifiers.map((modifier) => labels[modifier]), keyLabel(key)]
}

export function formatChord(chord: string, apple = isApplePlatform()): string[][] {
  return chordSteps(chord).map((step) => formatStroke(step, apple))
}

export function chordText(chord: string, apple = isApplePlatform()): string {
  return formatChord(chord, apple)
    .map((parts) => parts.join(apple ? '' : '+'))
    .join(' ')
}

const ARIA_MODIFIERS: Record<Modifier, string> = { mod: 'Control', ctrl: 'Control', alt: 'Alt', shift: 'Shift', meta: 'Meta' }
const ARIA_KEYS: Record<string, string> = {
  enter: 'Enter',
  escape: 'Escape',
  space: 'Space',
  tab: 'Tab',
  backspace: 'Backspace',
  delete: 'Delete',
  insert: 'Insert',
  home: 'Home',
  end: 'End',
  pageup: 'PageUp',
  pagedown: 'PageDown',
  arrowup: 'ArrowUp',
  arrowdown: 'ArrowDown',
  arrowleft: 'ArrowLeft',
  arrowright: 'ArrowRight',
  plus: 'Plus',
}

export function ariaKeyShortcut(chords: string[], apple = isApplePlatform()): string | undefined {
  const single = chords.filter((chord) => chordSteps(chord).length === 1)
  if (single.length === 0) return undefined
  return single
    .map((chord) => {
      const { modifiers, key } = strokeParts(canonicalStroke(chord, apple))
      const names = modifiers.map((modifier) => (modifier === 'mod' && apple ? 'Meta' : ARIA_MODIFIERS[modifier]))
      return [...names, ARIA_KEYS[key] ?? (isFunctionKey(key) ? key.toUpperCase() : key.toUpperCase())].join('+')
    })
    .join(' ')
}
