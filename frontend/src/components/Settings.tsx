import { useState, useEffect } from 'react'
import { check } from '@tauri-apps/plugin-updater'
import { ask, message } from '@tauri-apps/plugin-dialog'
import { relaunch } from '@tauri-apps/plugin-process'
import { getVersion } from '@tauri-apps/api/app'
import { signIn } from '@choochmeque/tauri-plugin-google-auth-api'
import { checkStatus } from '@choochmeque/tauri-plugin-biometry-api'
import {
  generateAndStoreKey,
  clearStoredKey
} from '../utils/biometricKeys'
import ConfirmationModal from './ConfirmationModal'
import AliasSettings from './AliasSettings'
import { useTheme, THEMES } from '../hooks/useTheme'
import { api } from '../api/client'
import { Google } from './ui/svgs/google'
import FingerprintIcon from './icons/FingerprintIcon'

type AuthMethod = 'oauth-pbkdf2' | 'oauth-argon2id' | 'biometric-keychain'

interface AuthPreferences {
  auth_method: string
  session_valid: boolean
  session_remaining_seconds: number
}

function getAuthMethodLabel(authMethod: string): string {
  switch (authMethod) {
    case 'oauth-pbkdf2':
    case 'oauth-argon2id':
      return 'Google OAuth'
    case 'biometric-keychain':
      return 'Biometric Authentication'
    default:
      return authMethod || 'Unknown'
  }
}

function Settings() {
  const [preferences, setPreferences] = useState<AuthPreferences>({
    auth_method: 'none',
    session_valid: false,
    session_remaining_seconds: 0
  })
  const [biometricAvailable, setBiometricAvailable] = useState(false)
  const [selectedMethod, setSelectedMethod] = useState<AuthMethod>('oauth-pbkdf2')
  const [liveRemainingSeconds, setLiveRemainingSeconds] = useState<number | null>(null)
  const [loading, setLoading] = useState(true)
  const [switching, setSwitching] = useState(false)
  const [error, setError] = useState('')
  const [confirmation, setConfirmation] = useState<{
    message: string
    onConfirm: () => void
  } | null>(null)
  const [appVersion, setAppVersion] = useState<string>('')
  const [checkingUpdate, setCheckingUpdate] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState(0)
  const [totalDownloadSize, setTotalDownloadSize] = useState(0)
  const [isDownloading, setIsDownloading] = useState(false)
  const { theme, setTheme } = useTheme()

  useEffect(() => {
    loadPreferences()
    loadVersion()
  }, [])

  useEffect(() => {
    if (liveRemainingSeconds === null || liveRemainingSeconds <= 0) return
    const interval = setInterval(() => {
      setLiveRemainingSeconds((prev) => {
        if (prev === null || prev <= 1) return 0
        return prev - 1
      })
    }, 1000)
    return () => clearInterval(interval)
  }, [liveRemainingSeconds])

  const loadPreferences = async () => {
    try {
      setLoading(true)
      setError('')

      const prefs = await api.getAuthPreferences()

      const authMethod = prefs.auth_method ?? 'none'
      const sessionValid = prefs.session_valid ?? false
      const remaining = prefs.session_remaining_seconds ?? 0
      setPreferences({
        auth_method: authMethod,
        session_valid: sessionValid,
        session_remaining_seconds: remaining
      })
      setLiveRemainingSeconds(sessionValid && remaining > 0 ? remaining : null)
      setSelectedMethod(
        authMethod === 'biometric-keychain' ? 'biometric-keychain' : authMethod === 'oauth-argon2id' ? 'oauth-argon2id' : 'oauth-pbkdf2'
      )

      const status = await checkStatus()
      setBiometricAvailable(status.isAvailable)
    } catch (err) {
      console.error('Failed to load preferences:', err)
      setError('Failed to load settings')
    } finally {
      setLoading(false)
    }
  }

  const loadVersion = async () => {
    try {
      const version = await getVersion()
      setAppVersion(version)
    } catch (err) {
      console.error('Failed to get version:', err)
    }
  }

  const checkForUpdates = async () => {
    try {
      setCheckingUpdate(true)
      setError('')

      const update = await check()

      if (update?.available) {
        const shouldUpdate = await ask(
          `Version ${update.version} is available. You're currently on version ${appVersion}. \n\nRelease notes:\n${update.body}\n\nWould you like to install this update?`,
          { title: 'Update Available', kind: 'info' }
        )

        if (shouldUpdate) {
          setIsDownloading(true)
          setDownloadProgress(0)
          setTotalDownloadSize(0)

          await update.downloadAndInstall((event) => {
            switch (event.event) {
              case 'Started':
                setTotalDownloadSize(event.data.contentLength || 0)
                break
              case 'Progress':
                setDownloadProgress((prev) => prev + (event.data.chunkLength || 0))
                break
              case 'Finished':
                setDownloadProgress((prev) => totalDownloadSize || prev)
                break
            }
          })

          await message('Update downloaded successfully! The app will now restart to install the update.', { kind: 'info' })
          await relaunch()
        }
      } else {
        await message(`You're already on the latest version (${appVersion}).`, { title: 'Up to Date', kind: 'info' })
      }
    } catch (err) {
      console.error('Failed to check for updates:', err)

      const errorMessage = err instanceof Error ? err.message : String(err)

      if (errorMessage.includes('Failed to fetch') || errorMessage.includes('Network') || errorMessage.includes('ECONNREFUSED')) {
        await message('Unable to check for updates. Please check your internet connection and try again.', { title: 'Network Error', kind: 'error' })
      } else {
        await message(`You're already on the latest version (${appVersion}).`, { title: 'Up to Date', kind: 'info' })
      }
    } finally {
      setCheckingUpdate(false)
      setIsDownloading(false)
    }
  }

  const hasChanges = selectedMethod !== preferences.auth_method

  const validateOAuthConfig = (): string | null => {
    const clientId = import.meta.env.VITE_GOOGLE_CLIENT_ID
    const clientSecret = import.meta.env.VITE_GOOGLE_CLIENT_SECRET
    if (
      !clientId ||
      !clientSecret ||
      clientId === 'your-client-id-here' ||
      clientSecret === 'your-client-secret-here'
    ) {
      return 'Google OAuth is not configured. Add VITE_GOOGLE_CLIENT_ID and VITE_GOOGLE_CLIENT_SECRET to your .env file.'
    }
    return null
  }

  const handleSave = () => {
    if (!hasChanges) return
    if (!preferences.session_valid) {
      setError('Vault must be unlocked to switch. Please unlock first.')
      return
    }
    if (selectedMethod === 'biometric-keychain') {
      if (!biometricAvailable) {
        setError('Biometric authentication is not configured on this device.')
        return
      }
      setConfirmation({
        message:
          'This will re-encrypt your vault with biometric authentication. Continue?',
        onConfirm: performSwitchToBiometric
      })
    } else {
      const oauthError = validateOAuthConfig()
      if (oauthError) {
        setError(oauthError)
        return
      }
      setConfirmation({
        message: 'This will re-encrypt your vault with Google OAuth. Continue?',
        onConfirm: performSwitchToOAuth
      })
    }
  }

  const performSwitchToBiometric = async () => {
    setConfirmation(null)
    setSwitching(true)
    setError('')
    try {
      const keyHex = await generateAndStoreKey()
      try {
        await api.reencryptVault(keyHex, 'biometric-keychain', '')
        await loadPreferences()
      } catch (reencryptErr) {
        await clearStoredKey()
        throw reencryptErr
      }
    } catch (err) {
      console.error('Failed to switch to biometric:', err)
      setError(String(err))
    } finally {
      setSwitching(false)
    }
  }

  const performSwitchToOAuth = async () => {
    setConfirmation(null)
    setSwitching(true)
    setError('')
    try {
      const response = await signIn({
        clientId: import.meta.env.VITE_GOOGLE_CLIENT_ID,
        clientSecret: import.meta.env.VITE_GOOGLE_CLIENT_SECRET,
        scopes: ['openid', 'email', 'profile'],
        successHtmlResponse:
          '<h1>Authentication successful! You can close this window.</h1>'
      })

      if (!response.idToken) {
        throw new Error('No ID token received from Google')
      }

      await api.reencryptVaultToOAuth(response.idToken)
      await clearStoredKey()
      await loadPreferences()
    } catch (err) {
      console.error('Failed to switch to OAuth:', err)
      setError(String(err))
    } finally {
      setSwitching(false)
    }
  }

  const handleCancel = () => {
    setSelectedMethod(
      preferences.auth_method === 'biometric-keychain'
        ? 'biometric-keychain'
        : 'oauth-pbkdf2'
    )
    setError('')
  }

  const getSessionTimeRemaining = () => {
    if (!preferences.session_valid) return null
    const remaining =
      liveRemainingSeconds !== null ? liveRemainingSeconds : preferences.session_remaining_seconds
    if (remaining <= 0) return 'Expired'
    const minutes = Math.floor(remaining / 60)
    const seconds = remaining % 60
    return `${minutes}m ${seconds}s`
  }

  if (loading) {
    return (
      <div className="flex items-center justify-center py-10 px-4">
        <div className="w-6 h-6 border-2 border-theme-accent border-t-transparent rounded-full animate-spin"></div>
        <span className="ml-3 text-theme-text font-theme">Loading settings...</span>
      </div>
    )
  }

  const currentLabel = getAuthMethodLabel(preferences.auth_method)

  return (
    <div className="px-5 py-5 animate-[settings-fade-in_0.3s_ease-out]">
      <div style={{ display: 'grid', gridTemplateColumns: '3fr 2fr', gap: '32px', position: 'relative' }}>
        <div className="flex flex-col gap-4 items-start">
          <header className="flex items-baseline justify-between gap-3 flex-wrap pb-2.5 border-b border-theme-border mb-2">
            <h2 className="font-theme text-2xl font-semibold tracking-wide text-theme-accent">Authentication</h2>
            <div className="flex items-center gap-2.5">
              <span className="text-[11px] font-medium text-theme-text-secondary bg-theme-surface px-2 py-1 uppercase tracking-wider">{currentLabel}</span>
              {preferences.session_valid && (
                <span className="font-theme text-[11px] text-theme-accent opacity-90">
                  {getSessionTimeRemaining()}
                </span>
              )}
            </div>
          </header>

          <div className="flex flex-col gap-2">
            <p className="text-[13px] text-theme-text-secondary leading-relaxed">
              Choose how you unlock your vault. Switching re-encrypts your data.
            </p>

            <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '10px' }}>
              <label
                className={`flex flex-col gap-1.5 p-5 border-2 border-theme-accent cursor-pointer transition-transform duration-100 shadow-theme relative ${
                  selectedMethod === 'biometric-keychain' ? 'bg-theme-accent' : 'bg-theme-bg'
                } ${!biometricAvailable ? 'opacity-50 cursor-not-allowed' : ''} ${
                  !biometricAvailable && selectedMethod !== 'biometric-keychain' ? '' :
                  selectedMethod !== 'biometric-keychain' ? 'hover:bg-theme-surface hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm' : ''
                }`}
              >
                <input
                  type="radio"
                  name="auth-method"
                  value="biometric-keychain"
                  checked={selectedMethod === 'biometric-keychain'}
                  onChange={() => setSelectedMethod('biometric-keychain')}
                  disabled={!biometricAvailable || switching}
                  className="absolute opacity-0 pointer-events-none"
                />
                <span className={`text-sm font-semibold inline-flex items-center gap-1.5 ${selectedMethod === 'biometric-keychain' ? 'text-theme-accent-text' : 'text-theme-text'}`}>
                  <FingerprintIcon size={17.6} className={selectedMethod === 'biometric-keychain' ? 'text-theme-accent-text' : 'text-theme-text'} />
                  Biometric
                </span>
                <span className={`text-xs ${selectedMethod === 'biometric-keychain' ? 'text-theme-accent-text opacity-80' : 'text-theme-text-secondary'}`}>
                  Fingerprint or face
                </span>
              </label>
              <label
                className={`flex flex-col gap-1.5 p-5 border-2 border-theme-accent cursor-pointer transition-transform duration-100 shadow-theme relative ${
                  selectedMethod === 'oauth-pbkdf2' || selectedMethod === 'oauth-argon2id' ? 'bg-theme-accent' : 'bg-theme-bg'
                } ${
                  selectedMethod !== 'oauth-pbkdf2' && selectedMethod !== 'oauth-argon2id' ? 'hover:bg-theme-surface hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm' : ''
                }`}
              >
                <input
                  type="radio"
                  name="auth-method"
                  value="oauth-pbkdf2"
                  checked={selectedMethod === 'oauth-pbkdf2' || selectedMethod === 'oauth-argon2id'}
                  onChange={() => setSelectedMethod('oauth-pbkdf2')}
                  disabled={switching}
                  className="absolute opacity-0 pointer-events-none"
                />
                <span className={`text-sm font-semibold inline-flex items-center gap-1.5 ${selectedMethod === 'oauth-pbkdf2' || selectedMethod === 'oauth-argon2id' ? 'text-theme-accent-text' : 'text-theme-text'}`}>
                  <Google className="w-4 h-4 flex-shrink-0" />
                  Google OAuth
                </span>
                <span className={`text-xs ${selectedMethod === 'oauth-pbkdf2' || selectedMethod === 'oauth-argon2id' ? 'text-theme-accent-text opacity-80' : 'text-theme-text-secondary'}`}>
                  Sign in with Google
                </span>
              </label>
            </div>

            {hasChanges && (
              <div className="flex gap-2 justify-end pt-1">
                <button
                  onClick={handleCancel}
                  disabled={switching}
                  className="px-5 py-2.5 bg-theme-bg text-theme-text border-2 border-theme-accent font-extrabold font-theme uppercase tracking-wider cursor-pointer transition-transform duration-100 hover:bg-theme-surface hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm active:translate-x-[4px] active:translate-y-[4px] active:shadow-none disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  Cancel
                </button>
                <button
                  onClick={handleSave}
                  disabled={switching}
                  className="px-5 py-2.5 bg-theme-accent text-theme-bg border-2 border-theme-accent font-extrabold font-theme uppercase tracking-wider cursor-pointer transition-transform duration-100 hover:bg-theme-text hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm active:translate-x-[4px] active:translate-y-[4px] active:shadow-none disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {switching ? 'Saving…' : 'Save'}
                </button>
              </div>
            )}

            {!biometricAvailable && preferences.auth_method !== 'biometric-keychain' && (
              <p className="text-xs text-theme-text-secondary -mt-1">Biometric authentication is not available on this device.</p>
            )}

            <p className="text-[11px] text-theme-text-secondary opacity-80 mt-1 pt-3 border-t border-theme-border">
              Vault is encrypted locally. No backup. Lost access = lost data.
            </p>
          </div>

          <div>
            <header className="flex items-baseline justify-between gap-3 flex-wrap pb-2.5 border-b border-theme-border mb-3">
              <h2 className="font-theme text-2xl font-semibold tracking-wide text-theme-accent">Appearance</h2>
            </header>
            <div className="flex flex-col gap-2">
              <p className="text-[13px] text-theme-text-secondary leading-relaxed">
                Choose the UI theme that suits your style. Changes apply instantly.
              </p>
              <div style={{ display: 'grid', gridTemplateColumns: 'repeat(2, 1fr)', gap: '8px', marginTop: '8px' }}>
                {THEMES.map((t) => (
                  <div
                    key={t.id}
                    onClick={() => setTheme(t.id)}
                    className={`flex items-center gap-1.5 px-2 py-1 border-2 border-theme-accent cursor-pointer transition-transform duration-100 min-w-[110px] ${
                      theme === t.id ? 'bg-theme-accent shadow-theme' : 'bg-theme-bg shadow-theme hover:bg-theme-surface hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm'
                    }`}
                  >
                    <div className="flex w-[14px] h-[14px] border border-white/15 overflow-hidden rounded-full flex-shrink-0">
                      <div className="flex-[3]" style={{ background: t.bg }} />
                      <div className="flex-1" style={{ background: t.primary }} />
                    </div>
                    <span className={`text-xs font-bold uppercase tracking-wider font-theme ${theme === t.id ? 'text-theme-accent-text' : 'text-theme-text'}`}>{t.name}</span>
                  </div>
                ))}
              </div>
            </div>
          </div>

          <AliasSettings />
        </div>

        <div className="flex flex-col gap-4 items-start">
          <div>
            <header className="flex items-baseline justify-between gap-3 flex-wrap pb-2.5 border-b border-theme-border mb-4">
              <h2 className="font-theme text-2xl font-semibold tracking-wide text-theme-accent">Updates</h2>
              <div className="flex items-center gap-2.5">
                <span className="text-[11px] font-medium text-theme-text-secondary bg-theme-surface px-2 py-1 uppercase tracking-wider">v{appVersion}</span>
              </div>
            </header>
            <div className="flex flex-col gap-2">
              <p className="text-[13px] text-theme-text-secondary leading-relaxed mb-3">
                Check for updates to get the latest features and security improvements.
              </p>
              <button
                onClick={checkForUpdates}
                disabled={checkingUpdate || isDownloading || switching}
                className="w-full px-5 py-2.5 bg-theme-accent text-theme-bg border-2 border-theme-accent font-extrabold font-theme uppercase tracking-wider cursor-pointer transition-transform duration-100 hover:bg-theme-text hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm active:translate-x-[4px] active:translate-y-[4px] active:shadow-none disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center"
              >
                {checkingUpdate ? 'Checking…' : isDownloading ? 'Downloading…' : 'Check for Updates'}
              </button>
              {isDownloading && totalDownloadSize > 0 && (
                <div className="mt-3">
                  <div className="flex justify-between text-xs opacity-80 mb-1">
                    <span>Downloading update...</span>
                    <span>{Math.round((downloadProgress / totalDownloadSize) * 100)}%</span>
                  </div>
                  <div className="w-full h-1 bg-theme-surface-hover rounded overflow-hidden">
                    <div
                      style={{
                        width: `${Math.min((downloadProgress / totalDownloadSize) * 100, 100)}%`,
                        height: '100%',
                        background: 'var(--color-theme-accent)',
                        transition: 'width 0.2s ease'
                      }}
                    />
                  </div>
                  <div className="text-[11px] opacity-60 mt-1">
                    {(downloadProgress / (1024 * 1024)).toFixed(1)} MB / {(totalDownloadSize / (1024 * 1024)).toFixed(1)} MB
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>

      {error && <div className="mt-3 px-3.5 py-3 bg-theme-danger text-theme-text text-[13px] leading-relaxed">{error}</div>}
      {confirmation && (
        <ConfirmationModal
          message={confirmation.message}
          onConfirm={confirmation.onConfirm}
          onCancel={() => setConfirmation(null)}
        />
      )}
    </div>
  )
}

export default Settings



