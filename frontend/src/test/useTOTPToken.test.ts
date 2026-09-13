import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest'
import { useTOTPToken } from '../hooks/useTOTPToken'
import { api } from '../api/client'

vi.mock('../api/client', () => ({
  api: { getTotpToken: vi.fn() },
}))

const getTotpToken = vi.mocked(api.getTotpToken)

async function flush() {
  await act(async () => {
    await Promise.resolve()
  })
}

describe('useTOTPToken', () => {
  beforeEach(() => {
    vi.useFakeTimers()
    getTotpToken.mockReset()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  test('fetches a token for the active entry', async () => {
    getTotpToken.mockResolvedValue({ token: '111111', remaining_seconds: 20 })

    const { result } = renderHook(() => useTOTPToken('entry-1'))
    await flush()

    expect(getTotpToken).toHaveBeenCalledWith('entry-1')
    expect(result.current.token).toBe('111111')
    expect(result.current.remainingSeconds).toBe(20)
  })

  test('does not fetch without an entry and clears stale state', async () => {
    const { result, rerender } = renderHook(
      ({ id }: { id: string | null }) => useTOTPToken(id),
      { initialProps: { id: 'entry-1' as string | null } }
    )
    getTotpToken.mockResolvedValue({ token: '111111', remaining_seconds: 30 })
    await flush()

    rerender({ id: null })
    await flush()

    expect(result.current.token).toBeNull()
    expect(result.current.remainingSeconds).toBe(0)

    getTotpToken.mockClear()
    await act(async () => {
      vi.advanceTimersByTime(5000)
    })
    expect(getTotpToken).not.toHaveBeenCalled()
  })

  test('counts down each second and refetches at the window boundary', async () => {
    getTotpToken
      .mockResolvedValueOnce({ token: '111111', remaining_seconds: 2 })
      .mockResolvedValueOnce({ token: '222222', remaining_seconds: 30 })

    const { result } = renderHook(() => useTOTPToken('entry-1'))
    await flush()
    expect(result.current.remainingSeconds).toBe(2)

    await act(async () => {
      vi.advanceTimersByTime(1000)
    })
    expect(result.current.remainingSeconds).toBe(1)

    await act(async () => {
      vi.advanceTimersByTime(1000)
    })
    expect(getTotpToken).toHaveBeenCalledTimes(2)
    expect(result.current.token).toBe('222222')
    expect(result.current.remainingSeconds).toBe(30)
  })

  test('stops polling when the selection changes away', async () => {
    getTotpToken.mockResolvedValue({ token: '111111', remaining_seconds: 30 })
    const { rerender } = renderHook(
      ({ id }: { id: string | null }) => useTOTPToken(id),
      { initialProps: { id: 'entry-1' as string | null } }
    )
    await flush()

    rerender({ id: 'entry-2' })
    getTotpToken.mockClear()
    await act(async () => {
      vi.advanceTimersByTime(5000)
    })

    expect(getTotpToken).not.toHaveBeenCalled()
  })

  test('stops polling after unmount', async () => {
    getTotpToken.mockResolvedValue({ token: '111111', remaining_seconds: 30 })
    const { unmount } = renderHook(() => useTOTPToken('entry-1'))
    await flush()

    unmount()
    getTotpToken.mockClear()
    await act(async () => {
      vi.advanceTimersByTime(5000)
    })

    expect(getTotpToken).not.toHaveBeenCalled()
  })

  test('clears the token when the backend rejects', async () => {
    getTotpToken.mockRejectedValue(new Error('Vault is locked'))

    const { result } = renderHook(() => useTOTPToken('entry-1'))
    await flush()

    expect(result.current.token).toBeNull()
    expect(result.current.remainingSeconds).toBe(0)
  })

  test('refreshes when the window regains focus or visibility', async () => {
    getTotpToken
      .mockResolvedValueOnce({ token: '111111', remaining_seconds: 30 })
      .mockResolvedValueOnce({ token: '222222', remaining_seconds: 30 })
      .mockResolvedValueOnce({ token: '333333', remaining_seconds: 30 })

    const { result } = renderHook(() => useTOTPToken('entry-1'))
    await flush()
    expect(getTotpToken).toHaveBeenCalledTimes(1)

    await act(async () => {
      window.dispatchEvent(new Event('focus'))
    })
    await flush()
    expect(getTotpToken).toHaveBeenCalledTimes(2)
    expect(result.current.token).toBe('222222')

    await act(async () => {
      document.dispatchEvent(new Event('visibilitychange'))
    })
    await flush()
    expect(getTotpToken).toHaveBeenCalledTimes(3)
    expect(result.current.token).toBe('333333')
  })
})
