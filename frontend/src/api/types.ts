import { z } from 'zod'

const OptionalStringSchema = z.string().nullable().optional()

export const CredentialSchema = z.object({
  id: z.string(),
  title: z.string(),
  username: z.string(),
  password: z.string(),
  url: OptionalStringSchema,
  icon_url: OptionalStringSchema,
  has_totp: z.boolean().optional(),
  alias_provider_id: OptionalStringSchema,
})
export type Credential = z.infer<typeof CredentialSchema>

export const CredentialPreviewSchema = CredentialSchema.omit({ password: true })
export type CredentialPreview = z.infer<typeof CredentialPreviewSchema>

export const SuccessResponseSchema = z.object({
  status: z.literal('success'),
  message: z.string().optional(),
})

export const ErrorResponseSchema = z.object({
  status: z.literal('error'),
  message: z.string(),
})

export const ResponseSchema = z.discriminatedUnion('status', [
  SuccessResponseSchema,
  ErrorResponseSchema,
])

export const SecretResponseSchema = z.discriminatedUnion('status', [
  z.object({ status: z.literal('success'), value: z.string() }),
  z.object({ status: z.literal('error'), message: z.string() }),
])

export const AuthMethodResponseSchema = z.object({
  status: z.string(),
  auth_method: z.string(),
})

export const VaultStatusResponseSchema = z.object({
  status: z.literal('success'),
  has_vault: z.boolean(),
  is_unlocked: z.boolean(),
})

export const SearchEntriesResponseSchema = z.object({
  status: z.literal('success'),
  entries: z.array(CredentialPreviewSchema),
})

export const FullEntryResponseSchema = z.object({
  status: z.literal('success'),
  entry: CredentialSchema,
})

export const AddEntryResponseSchema = z.object({
  status: z.literal('success'),
  id: z.string(),
})

export const TotpTokenResponseSchema = z.object({
  status: z.literal('success'),
  token: z.string(),
  remaining_seconds: z.number(),
})

export const AliasProviderSchema = z.object({
  provider_id: z.string(),
  description: OptionalStringSchema,
})
export type AliasProvider = z.infer<typeof AliasProviderSchema>

export const AliasConfigsResponseSchema = z.object({
  status: z.literal('success'),
  configs: z.array(AliasProviderSchema),
  default_provider_id: OptionalStringSchema,
})
export type AliasConfigsResponse = z.infer<typeof AliasConfigsResponseSchema>

export const EmailMaskResponseSchema = z.object({
  status: z.literal('success'),
  email: z.string(),
})

export const PasswordOptionsSchema = z.object({
  length: z.number().min(8).max(128),
  uppercase: z.boolean(),
  lowercase: z.boolean(),
  numbers: z.boolean(),
  symbols: z.boolean(),
  exclude_ambiguous: z.boolean(),
})
export type PasswordOptions = z.infer<typeof PasswordOptionsSchema>

export const StrengthReportSchema = z.object({
  score: z.number(),
  entropy: z.number(),
  label: z.string(),
  warnings: z.array(z.string()),
  suggestions: z.array(z.string()),
})
export type StrengthReport = z.infer<typeof StrengthReportSchema>

export const VaultHealthReportSchema = z.object({
  overall_score: z.number(),
  weak_passwords: z.array(z.any()),
  reused_passwords: z.array(z.any()),
  breached_credentials: z.array(z.any()),
  total_entries: z.number(),
  strong_passwords: z.number(),
  average_entropy: z.number(),
})
export type VaultHealthReport = z.infer<typeof VaultHealthReportSchema>

export type PaletteMode =
  | 'search'
  | 'actions'
  | 'add-entry'
  | 'edit-entry'
  | 'delete-confirm'
  | 'auth-selector'
  | 'oauth-setup'
  | 'oauth-login'
  | 'biometric-setup'
  | 'biometric-login'
  | 'migrate'
  | 'settings'
  | 'password-generator'
  | 'vault-health'
  | 'health-weak'
  | 'health-reused'
  | 'health-breached'


