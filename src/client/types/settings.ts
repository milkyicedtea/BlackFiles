export interface ApiKey {
  id: string
  label: string | null
  last_used_at: string | null
  created_at: string
}

export interface CreatedApiKey {
  id: string
  label: string | null
  key: string
  created_at: string
}

export interface ApiKeyListResponse {
  keys: Array<ApiKey>
}

export interface AdminApiKey extends ApiKey {
  user_id: string
  username: string
}

export interface AdminApiKeyListResponse {
  keys: Array<AdminApiKey>
}

export interface CreateApiKeyRequest {
  label: string | null
}

export interface PasswordFormValues {
  password: string
  confirmPassword: string
}
