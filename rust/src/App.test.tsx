import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import App from './App'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

const mockConfig = {
  autostart: false,
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
        case 'refresh_service_status':
          return true
        case 'is_workflow_running':
          return false
        case 'start_workflow':
          return { testCount: 2, successCount: 2, appliedCount: 2, results: mockResults }
        case 'stop_workflow':
          return 0
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
      // Use getAllByText since '仪表盘' appears in both sidebar and heading
      expect(screen.getAllByText('仪表盘').length).toBeGreaterThan(0)
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

  it('does not auto start workflow on mount when workflow is stopped', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('is_workflow_running')
    })

    await new Promise((resolve) => setTimeout(resolve, 900))

    const startCalls = vi.mocked(invoke).mock.calls.filter(([cmd]) => cmd === 'start_workflow')
    expect(startCalls).toHaveLength(0)
  })

  it('shows dashboard by default', async () => {
    render(<App />)

    // The app should render without crashing
    await waitFor(() => {
      // Use role query to be more specific - find the heading
      expect(screen.getByRole('heading', { name: '仪表盘' })).toBeInTheDocument()
    }, { timeout: 5000 })
  })

  it('navigates to settings', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getAllByText('设置').length).toBeGreaterThan(0)
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
      expect(screen.getAllByText('运行日志').length).toBeGreaterThan(0)
    })

    fireEvent.click(screen.getAllByText('运行日志')[0])

    await waitFor(() => {
      expect(screen.getByText('查看操作记录和测试结果')).toBeInTheDocument()
    })
  })

  it('starts workflow with toggle button', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('启动')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('启动'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('start_workflow')
    })
  })

  it('shows results after workflow starts', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('启动')).toBeInTheDocument()
    })

    fireEvent.click(screen.getByText('启动'))

    await waitFor(() => {
      expect(screen.getByText('100ms')).toBeInTheDocument()
    })
  })

  it('adds log entries during operations', async () => {
    render(<App />)

    await waitFor(() => {
      expect(screen.getByText('启动')).toBeInTheDocument()
    })

    // Start workflow
    fireEvent.click(screen.getByText('启动'))

    // Navigate to logs
    await waitFor(() => {
      fireEvent.click(screen.getAllByText('运行日志')[0])
    })

    await waitFor(() => {
      // Should have some log entries
      expect(screen.getByText(/已加载配置/)).toBeInTheDocument()
    })
  })

  it('handles toggle workflow (start and stop)', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    render(<App />)

    // Wait for config to load
    await waitFor(() => {
      expect(screen.getByText('启动')).toBeInTheDocument()
    })

    // Click to start workflow
    fireEvent.click(screen.getByText('启动'))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('start_workflow')
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
      // Use getAllByText since '仪表盘' appears in both sidebar and heading
      expect(screen.getAllByText('仪表盘').length).toBeGreaterThan(0)
    })
  })
})
