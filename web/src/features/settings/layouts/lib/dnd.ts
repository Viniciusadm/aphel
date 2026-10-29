export const PALETTE_ID = 'palette'

export function columnId(sectionId: string, column: number) {
  return `column|${sectionId}|${column}`
}

export function parseColumnId(id: string): { sectionId: string; column: number } | null {
  const [kind, sectionId, column] = id.split('|')
  return kind === 'column' && sectionId ? { sectionId, column: Number(column) } : null
}
