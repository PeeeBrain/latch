export interface OtpAuthDetails {
  secret: string
  title: string
  username: string
}

const nonEmpty = (value: string | undefined | null): string | undefined => {
  const trimmed = value?.trim()
  return trimmed ? trimmed : undefined
}

const hasUnsupportedParams = (params: URLSearchParams): boolean => {
  const algorithm = params.get('algorithm')
  const digits = params.get('digits')
  const period = params.get('period')
  return (
    (algorithm !== null && algorithm.toLowerCase() !== 'sha1') ||
    (digits !== null && digits !== '6') ||
    (period !== null && period !== '30')
  )
}

export function parseOtpAuthUri(input: string): OtpAuthDetails | null {
  const trimmed = input.trim()
  if (!trimmed.toLowerCase().startsWith('otpauth://')) return null

  let url: URL
  try {
    url = new URL(trimmed)
  } catch {
    return null
  }

  if (url.hostname.toLowerCase() !== 'totp') return null

  const secret = nonEmpty(url.searchParams.get('secret'))
  if (!secret) return null
  if (hasUnsupportedParams(url.searchParams)) return null

  let label: string
  try {
    label = decodeURIComponent(url.pathname.replace(/^\/+/, ''))
  } catch {
    return null
  }

  const separator = label.indexOf(':')
  const labelIssuer = nonEmpty(separator === -1 ? undefined : label.slice(0, separator))
  const labelAccount = nonEmpty(separator === -1 ? label : label.slice(separator + 1))

  return {
    secret,
    title: nonEmpty(url.searchParams.get('issuer')) ?? labelIssuer ?? labelAccount ?? '',
    username: separator === -1 ? '' : (labelAccount ?? ''),
  }
}
