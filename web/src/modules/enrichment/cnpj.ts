const FIRST_WEIGHTS = [5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]
const SECOND_WEIGHTS = [6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2]

function checkDigit(values: number[], weights: number[]): number {
  const sum = values.reduce((total, value, index) => total + value * weights[index], 0)
  const remainder = sum % 11
  return remainder < 2 ? 0 : 11 - remainder
}

export function normalizeCnpj(raw: string): string | null {
  const cleaned = raw.replace(/[./\-\s]/g, '').toUpperCase()
  if (!/^[0-9A-Z]{12}[0-9]{2}$/.test(cleaned)) return null
  if (/^(.)\1+$/.test(cleaned)) return null
  const values = [...cleaned].map((character) => character.charCodeAt(0) - 48)
  const first = checkDigit(values.slice(0, 12), FIRST_WEIGHTS)
  const second = checkDigit(values.slice(0, 13), SECOND_WEIGHTS)
  return values[12] === first && values[13] === second ? cleaned : null
}
