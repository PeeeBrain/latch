import { useState, useEffect } from 'react'
import { api } from '../api/client'
import { type AliasProvider } from '../api/types'
import { providerLabel } from '../utils/aliasProviders'

function errorMessage(err: unknown): string {
  return err instanceof Error ? err.message : String(err)
}

function AliasSettings() {
  const [configs, setConfigs] = useState<AliasProvider[]>([])
  const [defaultProviderId, setDefaultProviderId] = useState<string | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const [error, setError] = useState('')
  const [providerId, setProviderId] = useState('simplelogin')
  const [description, setDescription] = useState('')
  const [apiToken, setApiToken] = useState('')

  useEffect(() => {
    load()
  }, [])

  const load = async () => {
    try {
      setLoading(true)
      setError('')
      const result = await api.listAliasConfigs()
      setConfigs(result.configs)
      setDefaultProviderId(result.default_provider_id)
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setLoading(false)
    }
  }

  const handleSave = async () => {
    if (!apiToken.trim()) {
      setError('API access token is required')
      return
    }
    try {
      setSaving(true)
      setError('')
      await api.saveAliasConfig(providerId, apiToken.trim(), description.trim() || undefined)
      setApiToken('')
      setDescription('')
      await load()
    } catch (err) {
      setError(errorMessage(err))
    } finally {
      setSaving(false)
    }
  }

  const handleDelete = async (id: string) => {
    try {
      setError('')
      await api.deleteAliasConfig(id)
      await load()
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  const handleSetDefault = async (id: string) => {
    try {
      setError('')
      await api.setDefaultAliasProvider(id)
      await load()
    } catch (err) {
      setError(errorMessage(err))
    }
  }

  const inputClass =
    'w-full bg-theme-bg text-theme-text border-2 border-theme-accent px-3 py-2 font-theme text-[13px] outline-none focus:bg-theme-surface'

  return (
    <div>
      <header className="flex items-baseline justify-between gap-3 flex-wrap pb-2.5 border-b border-theme-border mb-3">
        <h2 className="font-theme text-2xl font-semibold tracking-wide text-theme-accent">
          Email Alias Integration
        </h2>
      </header>

      <div className="flex flex-col gap-2">
        <p className="text-[13px] text-theme-text-secondary leading-relaxed">
          Connect an alias provider to generate email masks for your credentials.
        </p>

        {loading ? (
          <p className="text-[13px] text-theme-text-secondary">Loading…</p>
        ) : (
          <ul className="flex flex-col gap-2 mt-1">
            {configs.length === 0 && (
              <li className="text-[13px] text-theme-text-secondary opacity-80">
                No alias providers configured.
              </li>
            )}
            {configs.map((config) => (
              <li
                key={config.provider_id}
                className="flex items-center justify-between gap-3 px-3 py-2 bg-theme-bg border-2 border-theme-accent shadow-theme-sm"
              >
                <div className="flex items-center gap-2 min-w-0">
                  <span className="text-[13px] font-semibold text-theme-text">{providerLabel(config.provider_id)}</span>
                  {config.description && (
                    <span className="text-[13px] text-theme-text-secondary truncate">{config.description}</span>
                  )}
                  {defaultProviderId === config.provider_id && (
                    <span className="text-[10px] font-bold tracking-wider text-theme-bg bg-theme-text px-2 py-0.5">
                      DEFAULT
                    </span>
                  )}
                </div>
                <div className="flex items-center gap-2 flex-shrink-0">
                  {defaultProviderId !== config.provider_id && (
                    <button
                      onClick={() => handleSetDefault(config.provider_id)}
                      className="text-[11px] uppercase tracking-wider font-bold px-2 py-1 bg-theme-bg text-theme-text border-2 border-theme-accent cursor-pointer hover:bg-theme-surface"
                    >
                      Make default
                    </button>
                  )}
                  <button
                    onClick={() => handleDelete(config.provider_id)}
                    className="text-[11px] uppercase tracking-wider font-bold px-2 py-1 bg-theme-danger text-theme-text border-2 border-theme-accent cursor-pointer"
                  >
                    Delete
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}

        <div className="flex flex-col gap-2 mt-2 pt-3 border-t border-theme-border">
          <label className="flex flex-col gap-1 text-[11px] uppercase tracking-wider text-theme-text-secondary">
            Provider type
            <select
              aria-label="Provider type"
              value={providerId}
              onChange={(e) => setProviderId(e.target.value)}
              className={inputClass}
            >
              <option value="simplelogin">SimpleLogin</option>
              <option value="duckduckgo">DuckDuckGo</option>
            </select>
          </label>
          <input
            value={description}
            onChange={(e) => setDescription(e.target.value)}
            placeholder="Description (optional)"
            className={inputClass}
          />
          <input
            type="password"
            value={apiToken}
            onChange={(e) => setApiToken(e.target.value)}
            placeholder="API access token"
            className={inputClass}
          />
          <button
            onClick={handleSave}
            disabled={saving}
            className="self-end px-5 py-2.5 bg-theme-accent text-theme-bg border-2 border-theme-accent font-extrabold font-theme uppercase tracking-wider cursor-pointer transition-transform duration-100 hover:bg-theme-text hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {saving ? 'Saving…' : 'Save integration'}
          </button>
        </div>

        {error && (
          <div className="mt-1 px-3.5 py-3 bg-theme-danger text-theme-text text-[13px] leading-relaxed">
            {error}
          </div>
        )}
      </div>
    </div>
  )
}

export default AliasSettings
