import { invoke } from '@tauri-apps/api/core'
import {
  SecretResponseSchema,
  ResponseSchema,
  AuthMethodResponseSchema,
  AddEntryResponseSchema,
  FullEntryResponseSchema,
  SearchEntriesResponseSchema,
  TotpTokenResponseSchema,
  VaultStatusResponseSchema,
  VaultHealthReportSchema,
  type Credential,
  type CredentialPreview,
  type PasswordOptions,
  type StrengthReport,
  type VaultHealthReport,
} from './types'

function parse<T>(result: unknown, schema: { parse: (v: unknown) => T }): T {
  return schema.parse(JSON.parse(result as string))
}

export const api = {
  // Vault lifecycle
  async provisionPassword(password: string): Promise<void> {
    const result = await invoke('init_vault', { password })
    parse(result, ResponseSchema)
  },

  async provisionOAuth(idToken: string): Promise<void> {
    const result = await invoke('init_vault_oauth', { idToken })
    parse(result, ResponseSchema)
  },

  async provisionWithKey(keyHex: string, kdf: string): Promise<void> {
    const result = await invoke('init_vault_with_key', { keyHex, kdf })
    parse(result, ResponseSchema)
  },

  async accessPassword(password: string): Promise<void> {
    const result = await invoke('unlock_vault', { password })
    parse(result, ResponseSchema)
  },

  async accessOAuth(idToken: string): Promise<void> {
    const result = await invoke('unlock_vault_oauth', { idToken })
    parse(result, ResponseSchema)
  },

  async accessKey(keyHex: string): Promise<void> {
    const result = await invoke('unlock_vault_with_key', { keyHex })
    parse(result, ResponseSchema)
  },

  async lockVault(): Promise<void> {
    await invoke('lock_vault')
  },

  async vaultStatus(): Promise<{ has_vault: boolean; is_unlocked: boolean }> {
    const result = await invoke('vault_status')
    const parsed = VaultStatusResponseSchema.parse(JSON.parse(result as string))
    return { has_vault: parsed.has_vault, is_unlocked: parsed.is_unlocked }
  },

  async getAuthMethod(): Promise<string> {
    const result = await invoke('get_vault_auth_method')
    return AuthMethodResponseSchema.parse(JSON.parse(result as string)).auth_method
  },

  // Credentials
  async searchEntries(query: string): Promise<CredentialPreview[]> {
    const result = await invoke('search_entries', { query })
    return SearchEntriesResponseSchema.parse(JSON.parse(result as string)).entries
  },

  async copyField(entryId: string, field: 'password' | 'username'): Promise<string> {
    const result = await invoke('request_secret', { entryId, field })
    const parsed = SecretResponseSchema.parse(JSON.parse(result as string))
    if (parsed.status === 'success') return parsed.value
    throw new Error(parsed.message)
  },

  async getFullEntry(entryId: string): Promise<Credential> {
    const result = await invoke('get_full_entry', { entryId })
    return FullEntryResponseSchema.parse(JSON.parse(result as string)).entry
  },

  async getTotpToken(entryId: string): Promise<{ token: string; remaining_seconds: number }> {
    const result = await invoke('get_totp_token', { entryId })
    const parsed = TotpTokenResponseSchema.parse(JSON.parse(result as string))
    return { token: parsed.token, remaining_seconds: parsed.remaining_seconds }
  },

  async addEntry(entry: {
    title: string; username: string; password: string;
    url?: string; iconUrl?: string; totpSecret?: string;
  }): Promise<string> {
    const result = await invoke('add_entry', entry)
    return AddEntryResponseSchema.parse(JSON.parse(result as string)).id
  },

  async updateEntry(entry: {
    id: string; title: string; username: string;
    password: string; url?: string; iconUrl?: string; totpSecret?: string;
  }): Promise<void> {
    const result = await invoke('update_entry', entry)
    parse(result, ResponseSchema)
  },

  async deleteEntry(entryId: string): Promise<void> {
    const result = await invoke('delete_entry', { entryId })
    parse(result, ResponseSchema)
  },

  // Password generator
  async generatePassword(options: PasswordOptions): Promise<string> {
    const result = await invoke('generate_password', { options })
    return JSON.parse(result as string).password
  },

  async analyzePassword(password: string): Promise<StrengthReport> {
    const result = await invoke('analyze_password_strength', { password })
    return JSON.parse(result as string).report as StrengthReport
  },

  // Vault health
  async checkVaultHealth(): Promise<VaultHealthReport> {
    const result = await invoke('check_vault_health')
    return VaultHealthReportSchema.parse(JSON.parse(result as string).report)
  },

  // Auth preferences
  async getAuthPreferences(): Promise<{
    auth_method: string; session_valid: boolean; session_remaining_seconds: number
  }> {
    const result = await invoke('get_auth_preferences')
    return JSON.parse(result as string)
  },

  // Re-encryption & migration
  async reencryptVault(newKeyHex: string, newKdf: string, newSalt: string): Promise<void> {
    const result = await invoke('reencrypt_vault', { newKeyHex, newKdf, newSalt })
    parse(result, ResponseSchema)
  },

  async reencryptVaultToOAuth(idToken: string): Promise<void> {
    const result = await invoke('reencrypt_vault_to_oauth', { idToken })
    parse(result, ResponseSchema)
  },

  async migrateToOAuth(password: string, idToken: string): Promise<void> {
    const result = await invoke('migrate_to_oauth', { password, idToken })
    parse(result, ResponseSchema)
  },
}


