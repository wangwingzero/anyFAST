export interface Endpoint {
  name: string
  url: string
  domain: string
  enabled: boolean
}

export interface EndpointResult {
  endpoint: Endpoint
  ip: string
  latency: number
  ttfb: number
  success: boolean
  error?: string
  warning?: string
  original_ip: string
  original_latency: number
  speedup_percent: number
  use_original: boolean
}

export interface AppConfig {
  endpoints: Endpoint[]
  autostart: boolean
  preferred_ips: string[]
}

export interface Progress {
  current: number
  total: number
  message: string
}

export interface LogEntry {
  level: 'success' | 'info' | 'warning' | 'error'
  message: string
  timestamp: string
}

// 历史记录模型
export interface HistoryRecord {
  timestamp: number
  domain: string
  original_latency: number
  optimized_latency: number
  speedup_percent: number
  applied: boolean
}

export interface HistoryStats {
  total_tests: number
  total_speedup_ms: number
  avg_speedup_percent: number
  records: HistoryRecord[]
}

// ===== Service 相关类型 =====

export interface PermissionStatus {
  hasPermission: boolean
  isUsingService: boolean
}

// ===== 更新相关类型 =====

export interface UpdateInfo {
  currentVersion: string
  latestVersion: string
  hasUpdate: boolean
  releaseUrl: string
  releaseNotes: string
  publishedAt: string
}
