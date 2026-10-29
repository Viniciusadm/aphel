export function normalizeText(value: string): string {
  return value.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase().trim()
}

export function matchesQuery(query: string, ...texts: (string | undefined)[]): boolean {
  const tokens = normalizeText(query).split(/\s+/).filter(Boolean)
  if (tokens.length === 0) return true
  const haystack = normalizeText(texts.filter(Boolean).join(' '))
  return tokens.every((token) => haystack.includes(token))
}
