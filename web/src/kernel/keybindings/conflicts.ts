import { canonicalChord, chordSteps } from './chord'
import { GLOBAL_SCOPE, type KeybindingScopeKind } from './types'

export type ConflictKind = 'duplicate' | 'prefix' | 'shadow' | 'reserved'
export type ConflictSeverity = 'error' | 'warning'

export interface ConflictCandidate {
  id: string
  scope: string
  keys: string[]
}

export interface KeybindingConflict {
  kind: ConflictKind
  severity: ConflictSeverity
  chord: string
  actionId: string
  otherId?: string
  otherChord?: string
}

const RESERVED_CHORDS = new Set([
  'mod+w',
  'mod+t',
  'mod+n',
  'mod+shift+n',
  'mod+shift+t',
  'mod+shift+w',
  'mod+l',
  'mod+r',
  'mod+shift+r',
  'mod+q',
  'mod+tab',
  'mod+shift+tab',
  'mod+p',
  'mod+f',
  'mod+c',
  'mod+v',
  'mod+x',
  'mod+z',
  'mod+a',
  'f5',
  'f11',
  'f12',
  'f8',
  'alt+f4',
  'alt+arrowleft',
  'alt+arrowright',
  'tab',
  'shift+tab',
])

export function isReservedChord(chord: string): boolean {
  return chordSteps(chord).length === 1 && RESERVED_CHORDS.has(chord)
}

function isPrefix(shorter: string, longer: string): boolean {
  const left = chordSteps(shorter)
  const right = chordSteps(longer)
  return left.length < right.length && left.every((step, index) => right[index] === step)
}

function relation(left: string, right: string): 'same' | 'prefix' | null {
  if (left === right) return 'same'
  if (isPrefix(left, right) || isPrefix(right, left)) return 'prefix'
  return null
}

function scopesInteract(
  left: string,
  right: string,
  kindOf: (scope: string) => KeybindingScopeKind | 'global',
): 'same' | 'shadow' | null {
  if (left === right) return 'same'
  const leftKind = kindOf(left)
  const rightKind = kindOf(right)
  if (leftKind === 'global' || rightKind === 'global') return 'shadow'
  return null
}

export function findConflicts(
  candidates: ConflictCandidate[],
  scopeKind: (scope: string) => KeybindingScopeKind | undefined,
): KeybindingConflict[] {
  const kindOf = (scope: string) => (scope === GLOBAL_SCOPE ? 'global' : (scopeKind(scope) ?? 'overlay'))
  const conflicts: KeybindingConflict[] = []
  const entries = candidates.flatMap((candidate) =>
    candidate.keys.map((key) => ({ candidate, chord: canonicalChord(key) })),
  )
  for (const { candidate, chord } of entries) {
    if (isReservedChord(chord)) {
      conflicts.push({ kind: 'reserved', severity: 'warning', chord, actionId: candidate.id })
    }
  }
  for (let leftIndex = 0; leftIndex < entries.length; leftIndex++) {
    for (let rightIndex = leftIndex + 1; rightIndex < entries.length; rightIndex++) {
      const left = entries[leftIndex]
      const right = entries[rightIndex]
      if (left.candidate.id === right.candidate.id) continue
      const keys = relation(left.chord, right.chord)
      if (!keys) continue
      const scopes = scopesInteract(left.candidate.scope, right.candidate.scope, kindOf)
      if (!scopes) continue
      const kind: ConflictKind = scopes === 'shadow' ? 'shadow' : keys === 'same' ? 'duplicate' : 'prefix'
      const severity: ConflictSeverity = kind === 'shadow' ? 'warning' : 'error'
      conflicts.push({ kind, severity, chord: left.chord, actionId: left.candidate.id, otherId: right.candidate.id, otherChord: right.chord })
      conflicts.push({ kind, severity, chord: right.chord, actionId: right.candidate.id, otherId: left.candidate.id, otherChord: left.chord })
    }
  }
  return conflicts
}

export function conflictsForChord(
  actionId: string,
  scope: string,
  chord: string,
  others: ConflictCandidate[],
  scopeKind: (scope: string) => KeybindingScopeKind | undefined,
): KeybindingConflict[] {
  const rest = others.filter((candidate) => candidate.id !== actionId)
  return findConflicts([...rest, { id: actionId, scope, keys: [chord] }], scopeKind).filter(
    (conflict) => conflict.actionId === actionId,
  )
}
