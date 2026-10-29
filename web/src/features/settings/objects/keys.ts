export function objectKeyFromLabel(label: string, maxLength: number): string {
  const normalized = label.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '')
  let key = ''
  for (const character of normalized) {
    const mapped = /[a-z]/.test(character) ? character : '_'
    if (mapped === '_' && (key === '' || key.endsWith('_'))) continue
    key += mapped
  }
  return key.slice(0, maxLength).replace(/_+$/, '')
}

export function relationKeyFromLabel(label: string, maxLength: number): string {
  const normalized = label.trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '')
  let key = ''
  for (const character of normalized) {
    const mapped = /[a-z0-9]/.test(character) ? character : '_'
    if (mapped === '_' && (key === '' || key.endsWith('_'))) continue
    if (key === '' && /[0-9]/.test(mapped)) continue
    key += mapped
  }
  return key.slice(0, maxLength).replace(/_+$/, '')
}
