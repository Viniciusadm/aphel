export const LIGHT_FOREGROUND = '#ffffff'
export const DARK_FOREGROUND = '#000000'
export const MIN_TEXT_CONTRAST = 4.5

const SOLID_TEXT_CONTRAST = 4.6
const LIGHT_SURFACE = '#fbfcfd'
const DARK_SURFACE = '#15181f'
const LIGHT_BRAND_MIN_DARK_CONTRAST = 10
const SEARCH_STEPS = 24

const HEX_COLOR = /^#[0-9a-f]{6}$/i

export function isHexColor(value: string | null | undefined): value is string {
  return typeof value === 'string' && HEX_COLOR.test(value)
}

type Rgb = [number, number, number]
type Oklch = { lightness: number; chroma: number; hue: number }

function toLinear(value: number): number {
  return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4
}

function fromLinear(value: number): number {
  return value <= 0.0031308 ? value * 12.92 : 1.055 * value ** (1 / 2.4) - 0.055
}

function hexToRgb(hex: string): Rgb {
  return [1, 3, 5].map((start) => Number.parseInt(hex.slice(start, start + 2), 16) / 255) as Rgb
}

function rgbToHex(rgb: Rgb): string {
  return `#${rgb
    .map((value) =>
      Math.round(Math.min(1, Math.max(0, value)) * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`
}

function hexToOklch(hex: string): Oklch {
  const [red, green, blue] = hexToRgb(hex).map(toLinear) as Rgb
  const long = Math.cbrt(0.4122214708 * red + 0.5363325363 * green + 0.0514459929 * blue)
  const medium = Math.cbrt(0.2119034982 * red + 0.6806995451 * green + 0.1073969566 * blue)
  const short = Math.cbrt(0.0883024619 * red + 0.2817188376 * green + 0.6299787005 * blue)
  const a = 1.9779984951 * long - 2.428592205 * medium + 0.4505937099 * short
  const b = 0.0259040371 * long + 0.7827717662 * medium - 0.808675766 * short
  return {
    lightness: 0.2104542553 * long + 0.793617785 * medium - 0.0040720468 * short,
    chroma: Math.hypot(a, b),
    hue: Math.atan2(b, a),
  }
}

function oklchToHex({ lightness, chroma, hue }: Oklch): string {
  const a = chroma * Math.cos(hue)
  const b = chroma * Math.sin(hue)
  const long = (lightness + 0.3963377774 * a + 0.2158037573 * b) ** 3
  const medium = (lightness - 0.1055613458 * a - 0.0638541728 * b) ** 3
  const short = (lightness - 0.0894841775 * a - 1.291485548 * b) ** 3
  const linear: Rgb = [
    4.0767416621 * long - 3.3077115913 * medium + 0.2309699292 * short,
    -1.2684380046 * long + 2.6097574011 * medium - 0.3413193965 * short,
    -0.0041960863 * long - 0.7034186147 * medium + 1.707614701 * short,
  ]
  return rgbToHex(linear.map((value) => fromLinear(Math.min(1, Math.max(0, value)))) as Rgb)
}

export function relativeLuminance(hex: string): number {
  const [red, green, blue] = hexToRgb(hex).map(toLinear) as Rgb
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

export function contrastRatio(first: string, second: string): number {
  const [lighter, darker] = [relativeLuminance(first), relativeLuminance(second)].sort((a, b) => b - a)
  return (lighter + 0.05) / (darker + 0.05)
}

function shiftUntilContrast(color: string, against: string, target: number, direction: 'darker' | 'lighter'): string {
  if (contrastRatio(color, against) >= target) return color
  const base = hexToOklch(color)
  let low = direction === 'darker' ? 0 : base.lightness
  let high = direction === 'darker' ? base.lightness : 1
  let best = direction === 'darker' ? oklchToHex({ ...base, lightness: 0 }) : oklchToHex({ ...base, lightness: 1 })
  for (let step = 0; step < SEARCH_STEPS; step += 1) {
    const lightness = (low + high) / 2
    const candidate = oklchToHex({ ...base, lightness })
    const passes = contrastRatio(candidate, against) >= target
    if (passes) best = candidate
    if (passes === (direction === 'darker')) low = lightness
    else high = lightness
  }
  return best
}

export interface BrandPalette {
  solid: string
  solidForeground: string
  lightText: string
  darkText: string
}

export function brandPalette(color: string): BrandPalette {
  const isLightBrand = contrastRatio(color, DARK_FOREGROUND) >= LIGHT_BRAND_MIN_DARK_CONTRAST
  const solid = isLightBrand ? color : shiftUntilContrast(color, LIGHT_FOREGROUND, SOLID_TEXT_CONTRAST, 'darker')
  return {
    solid,
    solidForeground: isLightBrand ? DARK_FOREGROUND : LIGHT_FOREGROUND,
    lightText: shiftUntilContrast(color, LIGHT_SURFACE, MIN_TEXT_CONTRAST, 'darker'),
    darkText: shiftUntilContrast(color, DARK_SURFACE, MIN_TEXT_CONTRAST, 'lighter'),
  }
}

export function formatRatio(ratio: number): string {
  return (Math.floor(ratio * 10) / 10).toFixed(1)
}
