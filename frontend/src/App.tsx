import { useState, useEffect, useRef } from 'react'
import { listen } from '@tauri-apps/api/event'
import CommandPalette from './components/CommandPalette'
import { useWindowAutoResize } from './hooks/useWindowAutoResize'
import { api } from './api/client'

type InitialMode =
  | 'auth-selector'
  | 'oauth-setup'
  | 'oauth-login'
  | 'password-setup'
  | 'password-login'
  | 'biometric-setup'
  | 'biometric-login'
  | 'search'

function App() {
  const [hasVault, setHasVault] = useState(false)
  const [isUnlocked, setIsUnlocked] = useState(false)
  const [authMethod, setAuthMethod] = useState<string>('none')
  const [loading, setLoading] = useState(true)
  const appRef = useRef<HTMLDivElement>(null)
  useWindowAutoResize(appRef)

  useEffect(() => {
    checkVaultStatus()
  }, [])

  useEffect(() => {
    const unlisten = listen('vault-locked', () => {
      checkVaultStatus()
    })
    return () => {
      unlisten.then((fn) => fn())
    }
  }, [])

  const checkVaultStatus = async () => {
    setLoading(true)
    try {
      const status = await api.vaultStatus()
      setHasVault(status.has_vault)
      setIsUnlocked(status.is_unlocked)

      if (status.has_vault && !status.is_unlocked) {
        const method = await api.getAuthMethod()
        setAuthMethod(method)
      }
    } catch (error) {
      console.error('Failed to check vault status:', error)
    } finally {
      setLoading(false)
    }
  }

  if (loading) {
    return (
      <div className="w-full p-0 bg-theme-bg relative z-1 overflow-y-auto overflow-x-hidden" ref={appRef}>
        <div className="w-full h-auto max-h-none bg-theme-bg border-none shadow-none flex flex-col overflow-visible">
          <div className="p-8 text-center text-theme-text font-bold tracking-wider uppercase">Loading...</div>
        </div>
      </div>
    )
  }

  const initialMode: InitialMode = !hasVault
    ? 'auth-selector'
    : !isUnlocked
      ? authMethod === 'biometric-keychain'
        ? 'biometric-login'
        : authMethod.startsWith('oauth-') ? 'oauth-login' : 'password-login'
      : 'search'

  return (
    <div className="w-full p-0 bg-theme-bg relative z-1 overflow-y-auto overflow-x-hidden" ref={appRef}>
      <CommandPalette initialMode={initialMode} />
    </div>
  )
}

export default App


