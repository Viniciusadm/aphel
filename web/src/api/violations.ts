import i18n from '@/i18n'

export const VIOLATION_MESSAGES_NAMESPACE = 'validation-messages'

type MessageValues = Record<string, string | number>

type Translation = [key: string, values?: MessageValues]

const LIST_SEPARATOR = /,\s*|\s+or\s+/

const PATTERNS: Array<[RegExp, (match: RegExpExecArray) => Translation]> = [
  [/^is required$/, () => ['required']],
  [/^must not be empty$/, () => ['required']],
  [/^is required and must have at most (\d+) characters$/, (match) => ['requiredTooLong', { max: Number(match[1]) }]],
  [/^must have at most (\d+) characters$/, (match) => ['tooLong', { max: Number(match[1]) }]],
  [/^must have (?:between )?(\d+) (?:and|to) (\d+) characters$/, (match) => ['lengthBetween', { min: Number(match[1]), max: Number(match[2]) }]],
  [/^must have at most (\d+) bytes$/, (match) => ['tooLarge', { max: Number(match[1]) }]],
  [/^must have at most (\d+) items$/, (match) => ['tooManyItems', { max: Number(match[1]) }]],
  [/^must be a valid id$|^must be a UUID$|^must be a (?:record|product) id$/, () => ['invalidId']],
  [/^must be a (?:valid )?user id$/, () => ['invalidUser']],
  [/^must be an integer between (-?\d+) and (-?\d+)$/, (match) => ['integerBetween', { min: match[1], max: match[2] }]],
  [/^must be between (-?[\d.]+)(?: \([^)]*\))? and (-?[\d.]+)(?: \([^)]*\))?$/, (match) => ['between', { min: match[1], max: match[2] }]],
  [/^must be a percentage between 0 and 100$/, () => ['percentage']],
  [/^must be a number$/, () => ['number']],
  [/^must be a non-negative (?:integer|number)(?: with at most (\d+) decimal places)?$/, (match) => ['nonNegative', { count: Number(match[1] ?? 0) }]],
  [/^must be a number greater than zero(?: with at most (\d+) decimal places)?$/, (match) => ['positive', { count: Number(match[1] ?? 0) }]],
  [/^must be (?:a boolean|true or false)$/, () => ['boolean']],
  [/^must be an object$/, () => ['object']],
  [/^must be a (?:non-empty )?string$|^must be a text value$/, () => ['text']],
  [/^must be a valid email address$/, () => ['email']],
  [/^must be an https URL|^must start with http:\/\/ or https:\/\/$|^must be a valid http or https URL$/, () => ['url']],
  [/^must be a hex color/, () => ['color']],
  [/^must be an ISO 4217 code with three letters$/, () => ['currency']],
  [/^must be a two-letter ISO country code$/, () => ['country']],
  [/^must be a valid domain/, () => ['domain']],
  [/^must be a date(?: in the YYYY-MM-DD format| in the format YYYY-MM-DD| \(YYYY-MM-DD\))?$/, () => ['date']],
  [/^must be an RFC 3339 (?:instant|date and time)$/, () => ['datetime']],
  [/^must not be in the future$/, () => ['notFuture']],
  [/^must not be before from/, () => ['dateOrder']],
  [/^must not be after to$/, () => ['dateOrder']],
  [/^must be snake_case|^must start with a (?:lowercase )?letter and (?:use|contain|have up to)/, () => ['key']],
  [/^is not a known field$|^is not a field of this record type$|^must be an active field of the record type$/, () => ['unknownField']],
  [/^is (?:duplicated|repeated|declared more than once)$/, () => ['duplicated']],
  [/^is already used by another (?:record|rule)$/, () => ['unique']],
  [/^is inactive$/, () => ['inactive']],
  [/^is read-only$/, () => ['readOnly']],
  [/^is not accessible$/, () => ['notAccessible']],
  [/^(?:user )?is not (?:a|an active) member of the organization$|^must be an active member of the organization$/, () => ['notMember']],
  [/^is not a role of the organization$/, () => ['unknownRole']],
  [/^must be one of the active lifecycle stages$/, () => ['lifecycleStage']],
  [/^must be an active lead distribution rule for this record type$/, () => ['distributionRule']],
  [/^must be a valid filter$/, () => ['filter']],
  [/^must be one of (.+)$/, (match) => ['oneOf', { values: match[1] }]],
  [/^must be ([a-z_]+(?:, [a-z_]+)* or [a-z_]+)$/, (match) => ['oneOf', { values: match[1].split(LIST_SEPARATOR).join(', ') }]],
]

export function violationTranslation(message: string): Translation | null {
  const text = message.trim()
  for (const [pattern, build] of PATTERNS) {
    const match = pattern.exec(text)
    if (match) return build(match)
  }
  return null
}

export function localizeViolationMessage(message: string): string {
  const translation = violationTranslation(message)
  if (!translation) return message
  const [key, values] = translation
  return i18n.t(key, { ns: VIOLATION_MESSAGES_NAMESPACE, defaultValue: message, ...values })
}
