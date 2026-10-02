import { useState, useEffect } from 'react'
import { api } from '../api/client'
import AliasSettings from './AliasSettings'

export default function MigrationSettings() {
  const [method, setMethod] = useState('')
  const [password, setPassword] = useState('')
  const [confirmation, setConfirmation] = useState('')
  const [busy, setBusy] = useState(false)
  const [message, setMessage] = useState('')
  useEffect(() => {
    let current = true
    api.getAuthMethod().then(value => { if (current) setMethod(value) }).catch(cause => { if (current) setMessage(String(cause)) })
    return () => { current = false }
  }, [])
  return <div className="p-5 flex flex-col gap-4">
    <h2>Prepare your vault for native Latch</h2>
    <p>Current access: {method || 'Loading...'}. Google access is retired in the GPUI app.</p>
    {method.startsWith('oauth-') && <p>Set a master password before installing the native release. Your credentials stay in the same encrypted vault file.</p>}
    <form className="flex flex-col gap-3" onSubmit={async event => {
      event.preventDefault()
      if (busy) return
      if (password.length < 12 || password !== confirmation) { setMessage('Use at least 12 characters and matching passwords.'); return }
      setBusy(true)
      setMessage('')
      const secret = password
      const repeated = confirmation
      setPassword('')
      setConfirmation('')
      try {
        await api.migrateToPassword(secret, repeated)
        setMethod(await api.getAuthMethod())
        setMessage('Your vault now uses a master password and can open in native Latch. Close this app before opening the native app.')
      } catch (cause) { setMessage(cause instanceof Error ? cause.message : String(cause)) }
      finally { setBusy(false) }
    }}>
      <label>New master password<input type="password" value={password} maxLength={1024} autoComplete="new-password" disabled={busy} onChange={event => setPassword(event.target.value)} className="w-full p-3 bg-theme-bg border border-theme-border" /></label>
      <label>Confirm master password<input type="password" value={confirmation} maxLength={1024} autoComplete="new-password" disabled={busy} onChange={event => setConfirmation(event.target.value)} className="w-full p-3 bg-theme-bg border border-theme-border" /></label>
      <p>There is no password recovery. Keep the new password somewhere safe.</p>
      <button type="submit" disabled={busy}>{busy ? 'Re-encrypting...' : 'Switch to master password'}</button>
    </form>
    {message && <p role="status">{message}</p>}
    <AliasSettings />
  </div>
}
