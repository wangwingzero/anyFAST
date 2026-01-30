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
  // 新增字段: 加速百分比显示 + 智能回退
  original_ip: string
  original_latency: number
  speedup_percent: number
  use_original: boolean
}

// 工作流结果 - 启动工作流后返回的统计信息
export interface WorkflowResult {
  testCount: number           // 测试的端点数
  successCount: number        // 成功的端点数
  appliedCount: number        // 应用的绑定数
  results: EndpointResult[]   // 详细测试结果
}

export interface AppConfig {
  endpoints: Endpoint[]
  autostart: boolean          // 开机自启动
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
  timestamp: number           // Unix 时间戳（秒）
  domain: string
  original_latency: number    // 原始延迟 (ms)
  optimized_latency: number   // 优化后延迟 (ms)
  speedup_percent: number     // 加速百分比
  applied: boolean            // 是否应用了优化
}

export interface HistoryStats {
  total_tests: number         // 总测试次数
  total_speedup_ms: number    // 累计节省时间 (ms)
  avg_speedup_percent: number // 平均加速百分比
  records: HistoryRecord[]    // 最近记录
}

// ===== 自动模式相关类型 =====

export interface HealthStatus {
  is_running: boolean
  last_check: number | null   // Unix 时间戳
  check_count: number
  switch_count: number
  endpoints_status: EndpointHealth[]
}

export interface EndpointHealth {
  domain: string
  current_ip: string | null
  best_ip: string | null       // 当前最优 IP
  best_latency: number         // 最优 IP 的延迟
  latency: number
  baseline_latency: number
  consecutive_failures: number
  is_healthy: boolean
}

export interface CheckResult {
  endpoints_health: EndpointHealth[]
  needs_switch: Endpoint[]
}

export interface SwitchResult {
  switched_count: number
  switched: SwitchedEndpoint[]
}

export interface SwitchedEndpoint {
  domain: string
  old_ip: string | null
  new_ip: string
  new_latency: number
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
