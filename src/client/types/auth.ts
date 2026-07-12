export interface User {
  id: string
  username: string
  password_hash?: string
  role_id: number
  role_name: string
  permissions?: Array<string>
  created_at: string
  updated_at: string
}

export interface LoginRequest {
  username: string
  password: string
}

export interface LoginResponse {
  user: User
}

export interface RoleWithPermissions {
  id: number
  name: string
  display_name: string
  position: number
  color: string
  permissions: Array<string>
  created_at: string
  updated_at: string
}

export interface Permission {
  id: number
  name: string
  display_name: string
  group_name: string
}

export interface FileEntry {
  name: string
  is_dir: boolean
  path: string
  size: number
  modified: number
}
