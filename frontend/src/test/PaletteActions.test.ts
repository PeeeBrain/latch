import { describe, it, expect, vi } from 'vitest'
import { createEntryActions, type EntryActionOptions } from '../components/PaletteActions'

function makeOptions(overrides: Partial<EntryActionOptions> = {}): EntryActionOptions {
  return {
    entryTitle: 'Example',
    hasTotp: false,
    onCopyPassword: vi.fn(),
    onCopyUsername: vi.fn(),
    onCopyTotp: vi.fn(),
    onEdit: vi.fn(),
    onLock: vi.fn(),
    onBack: vi.fn(),
    onDelete: vi.fn(),
    ...overrides,
  }
}

describe('createEntryActions', () => {
  it('exposes a Copy 2FA Code action only when TOTP is configured', () => {
    const withTotp = createEntryActions(makeOptions({ hasTotp: true }))
    expect(withTotp.map((action) => action.id)).toContain('copy-totp')

    const withoutTotp = createEntryActions(makeOptions({ hasTotp: false }))
    expect(withoutTotp.map((action) => action.id)).not.toContain('copy-totp')
  })

  it('routes the Copy 2FA Code action to its handler', async () => {
    const options = makeOptions({ hasTotp: true })
    const action = createEntryActions(options).find((item) => item.id === 'copy-totp')

    await action?.handler()

    expect(options.onCopyTotp).toHaveBeenCalledTimes(1)
  })
})
