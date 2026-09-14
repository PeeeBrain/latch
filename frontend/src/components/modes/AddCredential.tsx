import { useState, useEffect, useCallback } from 'react'
import { Globe, User, Key, ShieldCheck, Mail, ChevronDown } from 'lucide-react'
import { api } from '../../api/client'
import { fetchFavicon } from '../../utils/favicon'
import { parseOtpAuthUri } from '../../utils/otpauth'
import { providerLabel } from '../../utils/aliasProviders'
import PaletteInput from '../PaletteInput'
import { type PaletteMode, type CredentialPreview, type AliasProvider } from '../../api/types'

interface AddCredentialProps {
  editEntry: CredentialPreview | null
  prefillTitle: string
  generatedPassword: string
  onModeChange: (mode: PaletteMode) => void
  onCredentialsChanged: () => void
}

function AddCredential({ editEntry, prefillTitle, generatedPassword, onModeChange, onCredentialsChanged }: AddCredentialProps) {
  const isEditing = editEntry !== null
  const [formData, setFormData] = useState({
    title: prefillTitle || '',
    username: '',
    password: '',
    url: '',
    totpSecret: ''
  })
  const [error, setError] = useState('')
  const [loadedEdit, setLoadedEdit] = useState(false)
  const [totpTouched, setTotpTouched] = useState(false)
  const [aliasConfigs, setAliasConfigs] = useState<AliasProvider[]>([])
  const [defaultProviderId, setDefaultProviderId] = useState<string | null>(null)
  const [aliasMenuOpen, setAliasMenuOpen] = useState(false)
  const [generatingAlias, setGeneratingAlias] = useState(false)
  const [aliasProviderId, setAliasProviderId] = useState<string | null>(null)
  const [aliasError, setAliasError] = useState('')
  const [aliasSetupHint, setAliasSetupHint] = useState(false)

  useEffect(() => {
    if (isEditing && editEntry && !loadedEdit) {
      loadFullEntry()
    }
  }, [isEditing, editEntry, loadedEdit])

  useEffect(() => {
    api
      .listAliasConfigs()
      .then((result) => {
        setAliasConfigs(result.configs)
        setDefaultProviderId(result.default_provider_id)
      })
      .catch((err) => console.error('Failed to load alias providers:', err))
  }, [])

  useEffect(() => {
    if (generatedPassword) {
      setFormData((prev) => ({ ...prev, password: generatedPassword }))
    }
  }, [generatedPassword])

  const loadFullEntry = async () => {
    try {
      const fullEntry = await api.getFullEntry(editEntry!.id)
      setFormData({
        title: fullEntry.title,
        username: fullEntry.username,
        password: fullEntry.password,
        url: fullEntry.url || '',
        totpSecret: ''
      })
      setAliasProviderId(fullEntry.alias_provider_id ?? null)
      setLoadedEdit(true)
      setTotpTouched(false)
    } catch (error) {
      console.error('Failed to load entry for editing:', error)
    }
  }

  const handleTotpChange = (value: string) => {
    const parsed = parseOtpAuthUri(value)
    setTotpTouched(true)
    setFormData((prev) => ({
      ...prev,
      totpSecret: parsed?.secret ?? value,
      title: prev.title || parsed?.title || '',
      username: prev.username || parsed?.username || ''
    }))
  }

  const generateAlias = useCallback(async (providerId: string) => {
    try {
      setGeneratingAlias(true)
      setAliasError('')
      setAliasSetupHint(false)
      const email = await api.generateEmailMask(providerId)
      setFormData((prev) => ({ ...prev, username: email }))
      setAliasProviderId(providerId)
      setAliasMenuOpen(false)
    } catch (err) {
      setAliasError(err instanceof Error ? err.message : String(err))
    } finally {
      setGeneratingAlias(false)
    }
  }, [])

  const handleGenerateAliasShortcut = useCallback(() => {
    if (!defaultProviderId) {
      setAliasError('No default alias provider configured')
      setAliasSetupHint(true)
      return
    }
    generateAlias(defaultProviderId)
  }, [defaultProviderId, generateAlias])

  const handleSave = useCallback(async () => {
    setError('')

    if (!formData.title || !formData.username || !formData.password) {
      setError('Title, username, and password are required')
      return
    }

    try {
      let iconUrl: string | undefined
      const url = formData.url.trim() || undefined
      const totpSecret = totpTouched ? formData.totpSecret.trim() : undefined

      if (url) {
        try {
          const favicon = await fetchFavicon(url)
          if (favicon) {
            iconUrl = favicon
          }
        } catch (favError) {
          console.error('Error fetching favicon:', favError)
        }
      }

      const aliasProvider = aliasProviderId || undefined

      if (isEditing && editEntry) {
        await api.updateEntry({
          id: editEntry.id,
          title: formData.title,
          username: formData.username,
          password: formData.password,
          url,
          iconUrl,
          totpSecret,
          aliasProviderId: aliasProvider,
        })
      } else {
        await api.addEntry({
          title: formData.title,
          username: formData.username,
          password: formData.password,
          url,
          iconUrl,
          totpSecret,
          aliasProviderId: aliasProvider,
        })
      }

      setFormData({ title: '', username: '', password: '', url: '', totpSecret: '' })
      setTotpTouched(false)
      setAliasProviderId(null)
      setAliasError('')
      setAliasSetupHint(false)
      onCredentialsChanged()
      onModeChange('search')
    } catch (err) {
      console.error(`Error ${isEditing ? 'updating' : 'adding'} entry:`, err)
      setError(err instanceof Error ? err.message : String(err))
    }
  }, [formData, editEntry, isEditing, totpTouched, aliasProviderId, onModeChange, onCredentialsChanged])

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'e') {
        e.preventDefault()
        handleGenerateAliasShortcut()
      } else if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
        handleSave()
      } else if (e.key === 'Escape') {
        e.preventDefault()
        setFormData({ title: '', username: '', password: '', url: '', totpSecret: '' })
        setLoadedEdit(false)
        setTotpTouched(false)
        setAliasProviderId(null)
        setAliasError('')
        setAliasSetupHint(false)
        setAliasMenuOpen(false)
        onModeChange('search')
      }
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [handleSave, handleGenerateAliasShortcut, onModeChange])

  return (
    <>
      <PaletteInput
        value={formData.title}
        onChange={(val) => setFormData({ ...formData, title: val })}
        placeholder={isEditing ? 'Edit website title...' : 'Website title...'}
        icon={Globe}
        autoFocus={true}
      />
      <div className="relative">
        <PaletteInput
          value={formData.username}
          onChange={(val) => setFormData({ ...formData, username: val })}
          placeholder={isEditing ? 'Edit username or email...' : 'Username or email...'}
          icon={User}
          iconSpin={generatingAlias}
          hint={aliasProviderId ? providerLabel(aliasProviderId) : undefined}
          action={
            aliasConfigs.length > 0 ? (
              <div className="relative flex-shrink-0">
                <button
                  type="button"
                  aria-label="Generate email mask"
                  title="Generate email mask"
                  disabled={generatingAlias}
                  onClick={() => setAliasMenuOpen((open) => !open)}
                  className="flex items-center gap-1 justify-center h-11 px-3 bg-theme-bg border-2 border-theme-accent text-theme-text cursor-pointer transition-transform duration-100 hover:bg-theme-accent hover:text-theme-accent-text hover:translate-x-[1px] hover:translate-y-[1px] shadow-theme-sm disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <Mail size={16} />
                  <ChevronDown size={14} />
                </button>
                {aliasMenuOpen && (
                  <div className="absolute right-0 top-full mt-1 z-10 flex flex-col min-w-[180px] bg-theme-bg border-2 border-theme-accent shadow-theme">
                    {defaultProviderId && (
                      <button
                        type="button"
                        onClick={() => generateAlias(defaultProviderId)}
                        className="text-left px-3 py-2 text-[13px] font-theme text-theme-text bg-theme-bg cursor-pointer hover:bg-theme-accent hover:text-theme-accent-text border-b border-theme-border"
                      >
                        Default ({providerLabel(defaultProviderId)})
                      </button>
                    )}
                    {aliasConfigs.map((config) => (
                      <button
                        key={config.provider_id}
                        type="button"
                        onClick={() => generateAlias(config.provider_id)}
                        className="text-left px-3 py-2 text-[13px] font-theme text-theme-text bg-theme-bg cursor-pointer hover:bg-theme-accent hover:text-theme-accent-text border-b border-theme-border last:border-b-0"
                      >
                        {providerLabel(config.provider_id)}
                      </button>
                    ))}
                  </div>
                )}
              </div>
            ) : undefined
          }
        />
        {aliasError && (
          <div className="px-3 py-2 bg-theme-danger border-b border-red-900/20 text-theme-text text-sm flex items-center gap-3">
            <span>{aliasError}</span>
            {aliasSetupHint && (
              <button
                type="button"
                onClick={() => onModeChange('settings')}
                className="text-[11px] uppercase tracking-wider font-bold px-2 py-1 bg-theme-bg text-theme-text border-2 border-theme-accent cursor-pointer flex-shrink-0"
              >
                Open Settings
              </button>
            )}
          </div>
        )}
      </div>
      <PaletteInput
        value={formData.password}
        onChange={(val) => setFormData({ ...formData, password: val })}
        placeholder={isEditing ? 'Edit password...' : 'Password...'}
        type="password"
        icon={Key}
      />
      <PaletteInput
        value={formData.url}
        onChange={(val) => setFormData({ ...formData, url: val })}
        placeholder={isEditing ? 'Edit website URL...' : 'Website URL (optional)...'}
        icon={Globe}
      />
      <PaletteInput
        value={formData.totpSecret}
        onChange={handleTotpChange}
        placeholder={isEditing ? 'Edit 2FA secret or otpauth:// link (blank keeps current)...' : '2FA secret or otpauth:// link (optional)...'}
        icon={ShieldCheck}
        autoFocus={false}
        hint={isEditing && editEntry?.has_totp ? '2FA SET' : undefined}
      />
      {error && <div className="px-3 py-3 bg-theme-danger border-b border-red-900/20 text-theme-text text-sm">{error}</div>}
      <div className="px-3 py-2 border-t-2 border-theme-accent bg-theme-bg flex items-center justify-evenly w-full">
        <span className="text-[11px] text-theme-text-secondary inline-flex items-center gap-[5px] whitespace-nowrap">
          <kbd className="inline-block px-[5px] py-[2px] bg-theme-surface border border-theme-border font-theme text-[10px] font-medium text-theme-text-secondary">Enter</kbd> {isEditing ? 'Update' : 'Save'} <kbd className="inline-block px-[5px] py-[2px] bg-theme-surface border border-theme-border font-theme text-[10px] font-medium text-theme-text-secondary">Esc</kbd> Cancel {aliasConfigs.length > 0 && (
            <>
              <kbd className="inline-block px-[5px] py-[2px] bg-theme-surface border border-theme-border font-theme text-[10px] font-medium text-theme-text-secondary">Ctrl+E</kbd> Generate mask
            </>
          )}
        </span>
      </div>
    </>
  )
}

export default AddCredential



