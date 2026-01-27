import { vi } from 'vitest'
import type { AppConfig, EndpointResult, HistoryStats } from '../../types'

// Default mock config
export const mockConfig: AppConfig = {
  mode: 'auto',
  check_interval: 30,
  slow_threshold: 50,
  failure_threshold: 3,
  test_count: 3,
  minimize_to_tray: true,
  close_to_tray: true,
  clear_on_exit: false,
  cloudflare_ips: [],
  endpoints: [
    {
      name: 'Test Endpoint 1',
      url: 'https://test1.com/v1',
      domain: 'test1.com',
      enabled: true,
    },
    {
      name: 'Test Endpoint 2',
      url: 'https://test2.com/v1',
      domain: 'test2.com',
      enabled: true,
    },
  ],
}

// Mock endpoint results
export const mockResults: EndpointResult[] = [
  {
    endpoint: mockConfig.endpoints[0],
    ip: '1.2.3.4',
    latency: 100,
    ttfb: 100,
    success: true,
    original_ip: '5.6.7.8',
    original_latency: 200,
    speedup_percent: 50,
    use_original: false,
  },
  {
    endpoint: mockConfig.endpoints[1],
    ip: '2.3.4.5',
    latency: 150,
    ttfb: 150,
    success: true,
    original_ip: '6.7.8.9',
    original_latency: 250,
    speedup_percent: 40,
    use_original: false,
  },
]

// Mock failed result
export const mockFailedResult: EndpointResult = {
  endpoint: mockConfig.endpoints[0],
  ip: '1.2.3.4',
  latency: 9999,
  ttfb: 9999,
  success: false,
  error: 'Connection timeout',
  original_ip: '',
  original_latency: 0,
  speedup_percent: 0,
  use_original: false,
}

// Mock history stats
export const mockHistoryStats: HistoryStats = {
  total_tests: 10,
  total_speedup_ms: 1000,
  avg_speedup_percent: 45,
  records: [
    {
      timestamp: Date.now() / 1000 - 3600,
      domain: 'test1.com',
      original_latency: 200,
      optimized_latency: 100,
      speedup_percent: 50,
      applied: true,
    },
  ],
}

// Setup mock invoke responses
export async function setupMockInvoke() {
  const { invoke } = vi.mocked(await import('@tauri-apps/api/core'))

  invoke.mockImplementation(async (cmd: string, _args?: unknown) => {
    switch (cmd) {
      case 'get_config':
        return mockConfig
      case 'save_config':
        return undefined
      case 'start_speed_test':
        return mockResults
      case 'stop_speed_test':
        return undefined
      case 'apply_endpoint':
        return undefined
      case 'apply_all_endpoints':
        return mockResults.filter((r) => r.success).length
      case 'clear_all_bindings':
        return 2
      case 'get_bindings':
        return [
          ['test1.com', '1.2.3.4'],
          ['test2.com', '2.3.4.5'],
        ]
      case 'get_binding_count':
        return 2
      case 'check_admin':
        return true
      case 'get_history_stats':
        return mockHistoryStats
      default:
        console.warn(`Unhandled invoke command: ${cmd}`)
        return undefined
    }
  })

  return invoke
}

// Create custom mock invoke for specific test scenarios
export async function createMockInvoke(overrides: Record<string, unknown>) {
  const { invoke } = vi.mocked(await import('@tauri-apps/api/core'))

  invoke.mockImplementation(async (cmd: string, _args?: unknown) => {
    if (cmd in overrides) {
      const override = overrides[cmd]
      if (typeof override === 'function') {
        return override(_args)
      }
      return override
    }
    return (await setupMockInvoke()).mockImplementation
  })

  return invoke
}
