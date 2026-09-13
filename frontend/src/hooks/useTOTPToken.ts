import { useEffect, useState } from 'react'
import { api } from '../api/client'

export interface TotpTokenState {
  token: string | null
  remainingSeconds: number
}

const SUCCESS_TOKEN: TotpTokenState = { token: null, remainingSeconds: 0 }

export function useTOTPToken(entryId: string | null | undefined): TotpTokenState {
  const [state, setState] = useState<TotpTokenState>(SUCCESS_TOKEN)

  useEffect(() => {
    if (!entryId) {
      setState(SUCCESS_TOKEN)
      return
    }

    let cancelled = false
    let remaining = 0
    let refreshing = false

    const refresh = async () => {
      if (refreshing) return
      refreshing = true
      try {
        const result = await api.getTotpToken(entryId)
        if (cancelled) return
        remaining = result.remaining_seconds
        setState({ token: result.token, remainingSeconds: result.remaining_seconds })
      } catch {
        if (!cancelled) {
          remaining = 0
          setState(SUCCESS_TOKEN)
        }
      } finally {
        refreshing = false
      }
    }

    void refresh()

    const interval = window.setInterval(() => {
      if (remaining <= 1) {
        void refresh()
      } else {
        remaining -= 1
        setState((current) => ({ ...current, remainingSeconds: remaining }))
      }
    }, 1000)

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [entryId])

  return state
}
