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

export interface AppConfig {
  mode: 'manual' | 'auto'
  check_interval: number      // 健康检查间隔（秒）
  slow_threshold: number      // 慢速阈值（百分比，如 50 表示比基准慢 50%）
  failure_threshold: number   // 连续失败次数阈值
  test_count: number
  minimize_to_tray: boolean
  close_to_tray: boolean      // 关闭按钮最小化到托盘
  clear_on_exit: boolean      // 退出时清除 hosts 绑定
  cloudflare_ips: string[]
  endpoints: Endpoint[]
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
