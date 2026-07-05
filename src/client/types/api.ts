import type { FileEntry, Permission, RoleWithPermissions, User } from '@local/types/auth'

/** Paginated list response from list endpoints */
export interface PaginatedResponse<T> {
  data: Array<T>
  total: number
}

/** Response from GET /check */
export interface CheckResponse {
  user: User
}

/** Response from auth endpoints returning a user */
export interface UserResponse {
  user: User
}

/** Response shape for list endpoints */
export type UsersListResponse = PaginatedResponse<User>
export type RolesListResponse = PaginatedResponse<RoleWithPermissions>
export type DirectoryListResponse = PaginatedResponse<FileEntry>

/** Response shape for non-paginated collections */
export type PermissionsCollectionResponse = Array<Permission>
