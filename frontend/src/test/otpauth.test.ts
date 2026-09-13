import { describe, it, expect } from 'vitest'
import { parseOtpAuthUri } from '../utils/otpauth'

describe('parseOtpAuthUri', () => {
  it('returns null for a raw base32 secret', () => {
    expect(parseOtpAuthUri('JBSWY3DPEHPK3PXP')).toBeNull()
  })

  it('extracts secret, title and username from a labeled otpauth URI', () => {
    const result = parseOtpAuthUri(
      'otpauth://totp/ACME%20Co:john@example.com?secret=JBSWY3DPEHPK3PXP&issuer=ACME%20Co',
    )

    expect(result).toEqual({
      secret: 'JBSWY3DPEHPK3PXP',
      title: 'ACME Co',
      username: 'john@example.com',
    })
  })

  it('prefers the issuer parameter over the label prefix', () => {
    const result = parseOtpAuthUri(
      'otpauth://totp/Old%20Name:old@example.com?secret=JBSWY3DPEHPK3PXP&issuer=GitHub',
    )

    expect(result).toEqual({
      secret: 'JBSWY3DPEHPK3PXP',
      title: 'GitHub',
      username: 'old@example.com',
    })
  })

  it('uses a bare label as the title when no issuer is given', () => {
    const result = parseOtpAuthUri('otpauth://totp/My%20Bank?secret=JBSWY3DPEHPK3PXP')

    expect(result).toEqual({ secret: 'JBSWY3DPEHPK3PXP', title: 'My Bank', username: '' })
  })

  it('rejects otpauth URIs without a secret', () => {
    expect(parseOtpAuthUri('otpauth://totp/ACME%20Co?issuer=ACME')).toBeNull()
  })

  it('rejects non-totp and malformed URIs', () => {
    expect(parseOtpAuthUri('otpauth://hotp/ACME?secret=ABC')).toBeNull()
    expect(parseOtpAuthUri('otpauth://totp/%E0%A4%A?secret=ABC')).toBeNull()
  })
})
