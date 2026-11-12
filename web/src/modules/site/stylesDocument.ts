type Values = Record<string, unknown>

function isRecord(value: unknown): value is Values {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function slugOf(value: unknown): string | undefined {
  return isRecord(value) && typeof value.slug === 'string' ? value.slug : undefined
}

export function mergeStyles(base: unknown, over: unknown): unknown {
  if (isRecord(base) && isRecord(over)) {
    const result: Values = { ...base }
    for (const [key, value] of Object.entries(over)) result[key] = key in base ? mergeStyles(base[key], value) : value
    return result
  }
  if (Array.isArray(base) && Array.isArray(over) && [...base, ...over].every((entry) => slugOf(entry))) {
    const result = base.map((entry) => {
      const replacement = over.find((item) => slugOf(item) === slugOf(entry))
      return replacement ? mergeStyles(entry, replacement) : entry
    })
    for (const entry of over) if (!base.some((item) => slugOf(item) === slugOf(entry))) result.push(entry)
    return result
  }
  return over === undefined ? base : over
}
