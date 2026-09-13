import { renderHook, act } from '@testing-library/react'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { useClipboardGuard } from '../hooks/useClipboardGuard'

describe('useClipboardGuard', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    vi.mocked(navigator.clipboard.writeText).mockReset()
    vi.mocked(navigator.clipboard.readText).mockReset()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('clears the clipboard after 30s when the copied value is still there', async () => {
    vi.mocked(navigator.clipboard.readText).mockResolvedValue('secret')
    const { result } = renderHook(() => useClipboardGuard())

    await act(async () => {
      await result.current.copy('secret')
    })
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith('secret')

    await act(async () => {
      vi.advanceTimersByTime(30_000)
      await Promise.resolve()
    })

    expect(navigator.clipboard.writeText).toHaveBeenLastCalledWith('')
  })

  it('leaves the clipboard untouched when the user copied something else', async () => {
    vi.mocked(navigator.clipboard.readText).mockResolvedValue('something else')
    const { result } = renderHook(() => useClipboardGuard())

    await act(async () => {
      await result.current.copy('secret')
    })
    await act(async () => {
      vi.advanceTimersByTime(30_000)
      await Promise.resolve()
    })

    expect(navigator.clipboard.writeText).toHaveBeenCalledTimes(1)
  })
})
