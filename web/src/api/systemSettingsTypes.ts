import type { RequestLogRetentionSettings } from './requestLogRetention'

export interface SystemSettings {
  requestRateLimit: number
  authTokenLogRetentionDays: number
  mcpSessionAffinityKeyCount: number
  rebalanceMcpEnabled: boolean
  rebalanceMcpSessionPercent: number
  apiRebalanceEnabled: boolean
  apiRebalancePercent: number
  rechargeFeatureEnabled: boolean
  rechargeUserEnabled: boolean
  adminDefaultActiveUsersOnly: boolean
  userBlockedKeyBaseLimit: number
  globalIpLimit: number
  trustedProxyCidrs: string[]
  trustedClientIpHeaders: string[]
  requestLogRetention: RequestLogRetentionSettings
}

export interface AdminUserListStats {
  activeUsers90d: number
  totalUsers: number
  windowDays: number
}

export interface ForwardProxySettingsEnvelope {
  forwardProxy?: import('./runtime').ForwardProxySettings | null
  systemSettings?: SystemSettings | null
  adminUserListStats?: AdminUserListStats | null
}

export interface UpdateSystemSettingsPayload {
  requestRateLimit: number
  authTokenLogRetentionDays: number
  mcpSessionAffinityKeyCount: number
  rebalanceMcpEnabled: boolean
  rebalanceMcpSessionPercent: number
  apiRebalanceEnabled: boolean
  apiRebalancePercent: number
  rechargeFeatureEnabled: boolean
  rechargeUserEnabled: boolean
  adminDefaultActiveUsersOnly: boolean
  trustedProxyCidrs: string[]
  trustedClientIpHeaders: string[]
  userBlockedKeyBaseLimit: number
  globalIpLimit: number
  requestLogRetention: RequestLogRetentionSettings
}
