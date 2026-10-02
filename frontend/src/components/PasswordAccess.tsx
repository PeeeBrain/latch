import { useState } from 'react'
import { api } from '../api/client'

export default function PasswordAccess({ setup = false, onSuccess, onError }: {
  setup?: boolean
  onSuccess: () => void
  onError: (error: string) => void
}) {
  const [password, setPassword] = useState('')
  const [confirmation, setConfirmation] = useState('')
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  return <form className="p-5 flex flex-col gap-4" onSubmit={async event => {
    event.preventDefault()
    if (busy) return
    if (setup && (password.length < 12 || password !== confirmation)) {
      setError('Use at least 12 characters and matching passwords.')
      return
    }
    setBusy(true)
    const secret = password
    setPassword('')
    setConfirmation('')
    try {
      if (setup) await api.provisionPassword(secret)
      else await api.accessPassword(secret)
      onSuccess()
    } catch (cause) {
      const message = cause instanceof Error ? cause.message : String(cause)
      setError(message)
      onError(message)
    } finally { setBusy(false) }
  }}>
    <h2>{setup ? 'Create a password vault' : 'Unlock with your master password'}</h2>
    <label>Master password<input autoFocus type="password" value={password} maxLength={1024} autoComplete={setup ? 'new-password' : 'current-password'} onChange={event => setPassword(event.target.value)} disabled={busy} className="w-full p-3 bg-theme-bg border border-theme-border" /></label>
    {setup && <label>Confirm password<input type="password" value={confirmation} maxLength={1024} autoComplete="new-password" onChange={event => setConfirmation(event.target.value)} disabled={busy} className="w-full p-3 bg-theme-bg border border-theme-border" /></label>}
    {setup && <p>There is no password recovery. Keep your master password somewhere safe.</p>}
    {error && <p role="alert">{error}</p>}
    <button disabled={busy} type="submit">{busy ? 'Working...' : setup ? 'Create vault' : 'Unlock'}</button>
  </form>
}
