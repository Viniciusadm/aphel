type Lookup = (name: string) => string | undefined

const HEX = /^#([0-9a-f]{3,4}|[0-9a-f]{6}|[0-9a-f]{8})$/i
const VARIABLE = /^var\(--([a-z0-9-]+)\)$/
const FUNCTION = /^(rgb|rgba|hsl|hsla|oklch)\(\s*([^)]*)\)$/i
const MAX_REFERENCE_DEPTH = 4

function clamp(value: number): number {
  return Math.min(1, Math.max(0, value))
}

function toHex(red: number, green: number, blue: number): string {
  return `#${[red, green, blue]
    .map((channel) =>
      Math.round(clamp(channel) * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`
}

function fromLinear(value: number): number {
  return value <= 0.0031308 ? value * 12.92 : 1.055 * value ** (1 / 2.4) - 0.055
}

function oklchToHex(lightness: number, chroma: number, hue: number): string {
  const radians = (hue * Math.PI) / 180
  const a = chroma * Math.cos(radians)
  const b = chroma * Math.sin(radians)
  const long = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3
  const medium = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3
  const short = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3
  return toHex(
    fromLinear(clamp(4.0767416621 * long - 3.3077115913 * medium + 0.2309699292 * short)),
    fromLinear(clamp(-1.2684380046 * long + 2.6097574011 * medium - 0.3413193965 * short)),
    fromLinear(clamp(-0.0041960863 * long - 0.7034186147 * medium + 1.707614701 * short)),
  )
}

function hslToHex(hue: number, saturation: number, lightness: number): string {
  const amount = saturation * Math.min(lightness, 1 - lightness)
  const channel = (offset: number) => {
    const position = (offset + hue / 30) % 12
    return lightness - amount * Math.max(-1, Math.min(position - 3, 9 - position, 1))
  }
  return toHex(channel(0), channel(8), channel(4))
}

function numbers(body: string): number[] {
  return body
    .split(/[\s,/]+/)
    .filter(Boolean)
    .map((part) => {
      const value = Number.parseFloat(part)
      return part.endsWith('%') ? value / 100 : value
    })
}

function expandHex(hex: string): string {
  if (hex.length === 4 || hex.length === 5) {
    return `#${[...hex.slice(1, 4)].map((digit) => digit + digit).join('')}`
  }
  return hex.slice(0, 7)
}

export function cssColorToHex(value: string, lookup: Lookup, depth = 0): string | null {
  const text = value.trim()
  if (HEX.test(text)) return expandHex(text.toLowerCase())
  const reference = VARIABLE.exec(text)
  if (reference) {
    const target = lookup(reference[1])
    return target && depth < MAX_REFERENCE_DEPTH ? cssColorToHex(target, lookup, depth + 1) : null
  }
  const match = FUNCTION.exec(text)
  if (!match) return null
  const [first = Number.NaN, second = Number.NaN, third = Number.NaN] = numbers(match[2])
  if ([first, second, third].some(Number.isNaN)) return null
  switch (match[1].toLowerCase()) {
    case 'oklch':
      return oklchToHex(first, second, third)
    case 'hsl':
    case 'hsla':
      return hslToHex(first, second, third)
    default: {
      const scale = match[2].includes('%') ? 1 : 255
      return toHex(first / scale, second / scale, third / scale)
    }
  }
}
