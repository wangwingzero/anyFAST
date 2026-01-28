import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import App from './App'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockConfig = {
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
    { name: 'Test 1', url: 'https://test1.com/v1', domain: 'test1.com', enabled: true },
    { name: 'Test 2', url: 'https://test2.com/v1', domain: 'test2.com', enabled: true },
  ],
}

const mockResults = [
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
]

describe('App', () => {
  beforeEach(async () => {
    vi.clearAllMocks()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      switch (cmd) {
        case 'get_config':
          return mockConfig
        case 'get_binding_count':
          return 0
        case 'check_admin':
          return true
        case 'get_permission_status':
          return { hasPermission: true, isUsingService: false }
        case 'start_speed_test':
          return mockResults
        case 'apply_all_endpoints':
          return 1
        case 'clear_all_bindings':
          return 0
        default:
          return undefined
      }
    })
  })

  it('renders main layout', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('anyFAST')).toBeInTheDocument()
    })
  })

  it('loads config on mount', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_config')
    })
  })

  it('loads binding count on mount', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('get_binding_count')
    })
  })

  it('shows dashboard by default', async () => {
    render(<App />)

    // The app should render without crashing
    // Sidebar always renders anyFAST logo
    await waitFor(() => {
      expect(screen.getByText('anyFAST')).toBeInTheDocument()
    }, { timeout: 5000 })
  })

  it('navigates to settings', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('设置')).toBeInTheDocument()
    })

    // Click the settings nav item (in sidebar)
    const settingsNav = screen.getAllByText('设置')[0]
    fireEvent.click(settingsNav)

    await waitFor(() => {
      expect(screen.getByText('配置运行参数')).toBeInTheDocument()
    })
  })

  it('navigates to logs', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('运行日志')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('运行日志'))

    await waitFor(() => {
      expect(screen.getByText('查看操作记录和测试结果')).toBeInTheDocument()
    })
  })

  it('starts speed test', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('开始测速')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('开始测速'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('start_speed_test')
    })
  })

  it('shows results after test', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('开始测速')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('开始测速'))

    await waitFor(() => {
      expect(screen.getByText('100ms')).toBeInTheDocument()
    })
  })

  it('adds log entries during operations', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('开始测速')).toBeInTheDocument()
    })

    // Start test
    fireEvent.click(screen.getByText('开始测速'))

    // Navigate to logs
    await waitFor(() => {
      fireEvent.click(screen.getByText('运行日志'))
    })

    await waitFor(() => {
      // Should have some log entries
      expect(screen.getByText(/已加载配置/)).toBeInTheDocument()
    })
  })

  it('handles apply all endpoints', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    // Wait for config to load and run test
    await waitFor(() => {
      expect(screen.getByText('开始测速')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('开始测速'))

    await waitFor(() => {
      expect(screen.getByText('一键全部应用')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('一键全部应用'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('apply_all_endpoints')
    })
  })

  it('handles clear bindings', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('清除绑定')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('清除绑定'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('clear_all_bindings')
    })
  })

  it('handles config load error gracefully', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_config') {
        throw new Error('Config load failed')
      }
      if (cmd === 'get_permission_status') {
        return { hasPermission: true, isUsingService: false }
      }
      return undefined
    })

    // Should not crash
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('anyFAST')).toBeInTheDocument()
    })
  })
})
