import type { ValidationIssue } from './validation'

const SERVER_MESSAGES: Array<[RegExp, (match: RegExpExecArray) => ValidationIssue]> = [
  [/^is required$/, () => ({ code: 'required' })],
  [/^must be a string$/, () => ({ code: 'invalid' })],
  [/^must be a number$/, () => ({ code: 'invalidNumber' })],
  [/^must be true or false$/, () => ({ code: 'boolean' })],
  [/^must be a list of option values$/, () => ({ code: 'option' })],
  [/^must have at most (\d+) characters$/, (match) => ({ code: 'tooLong', values: { max: Number(match[1]) } })],
  [/^does not match the expected format$/, () => ({ code: 'pattern' })],
  [/^must be a valid email address$/, () => ({ code: 'email' })],
  [/^must be a valid http or https URL$/, () => ({ code: 'url' })],
  [/^must be a valid phone number$/, () => ({ code: 'phone' })],
  [/^must be a valid ([a-z_]+) value$/, (match) => ({ code: 'mask', values: { context: match[1] } })],
  [/^must be a date in the format/, () => ({ code: 'date' })],
  [/^must be an RFC 3339 date and time$/, () => ({ code: 'datetime' })],
  [/^must be greater than or equal to (.+)$/, (match) => ({ code: 'min', values: { min: match[1] } })],
  [/^must be less than or equal to (.+)$/, (match) => ({ code: 'max', values: { max: match[1] } })],
  [/^must have at most (\d+) decimal places?$/, (match) => ({ code: 'decimals', values: { count: Number(match[1]) } })],
  [/^is out of the supported range$/, () => ({ code: 'outOfRange' })],
  [/^option .* does not exist$/, () => ({ code: 'option' })],
  [/^option .* is inactive$/, () => ({ code: 'inactiveOption' })],
  [/^must have at most (\d+) options$/, (match) => ({ code: 'tooManyOptions', values: { max: Number(match[1]) } })],
  [/^must be a valid id$/, () => ({ code: 'reference' })],
  [/^references a record that does not exist or that you cannot read$/, () => ({ code: 'referenceNotFound' })],
  [/^is already used by another record$/, () => ({ code: 'unique' })],
  [/^is not a known field$/, () => ({ code: 'unknownField' })],
  [/^is inactive$/, () => ({ code: 'inactiveField' })],
  [/^is not accessible$/, () => ({ code: 'notAccessible' })],
  [/^is read-only$/, () => ({ code: 'readOnlyField' })],
]

export function serverMessageIssue(message: string): ValidationIssue {
  for (const [pattern, build] of SERVER_MESSAGES) {
    const match = pattern.exec(message.trim())
    if (match) return build(match)
  }
  return { code: 'invalid' }
}
