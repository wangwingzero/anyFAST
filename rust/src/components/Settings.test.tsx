import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent, waitFor } from '@testing-library/react'
import { Settings } from './Settings'
import type { Endpoint, AppConfig } from '../types'

// Mock Tauri invoke
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}))

// Mock Tauri updater plugin
vi.mock('@tauri-apps/plugin-updater', () => ({
  check: vi.fn(),
}))

// Mock Tauri process plugin
vi.mock('@tauri-apps/plugin-process', () => ({
  relaunch: vi.fn(),
}))

describe('Settings', () => {
  const mockProxySnapshot = {
    httpProxy: 'http://127.0.0.1:7890',
    httpsProxy: 'http://127.0.0.1:7890',
    allProxy: undefined,
    noProxy: 'localhost,127.0.0.1,::1',
    detectedSystemProxy: 'http://127.0.0.1:7890',
    localXrayProxy: 'http://127.0.0.1:10808',
    localXrayAvailable: true,
  }

  const mockEndpoints: Endpoint[] = [
    { name: 'Test 1', url: 'https://test1.com/v1', domain: 'test1.com', enabled: true },
    { name: 'Test 2', url: 'https://test2.com/v1', domain: 'test2.com', enabled: false },
  ]

  const mockConfig: AppConfig = {
    check_interval: 120,
    slow_threshold: 150,
    failure_threshold: 5,
    test_count: 3,
    endpoints: mockEndpoints,
    autostart: false,
    preferred_ips: [],
    continuous_mode: true,
    test_aggressiveness: 2,
    update_proxy: 'auto',
  }

  const defaultProps = {
    config: mockConfig,
    onEndpointsChange: vi.fn(),
    onConfigChange: vi.fn(),
  }

  beforeEach(async () => {
    vi.clearAllMocks()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'get_current_version') {
        return '2.6.3'
      }
      if (cmd === 'get_autostart') {
        return false
      }
      if (cmd === 'detect_system_proxy') {
        return 'http://127.0.0.1:7890'
      }
      if (cmd === 'get_proxy_env_snapshot') {
        return mockProxySnapshot
      }
      return undefined
    })
  })

  const renderSettings = async (props = defaultProps) => {
    render(<Settings {...props} />)
    await screen.findByText('anyFAST v2.6.3')
  }

  it('renders header correctly', async () => {
    await renderSettings()

    expect(screen.getByText('设置')).toBeInTheDocument()
    expect(screen.getByText('配置运行参数')).toBeInTheDocument()
  })

  it('shows system section with autostart toggle', async () => {
    await renderSettings()

    expect(screen.getByText('系统')).toBeInTheDocument()
    expect(screen.getByText('开机自启动')).toBeInTheDocument()
    expect(screen.getByText('系统启动时自动运行 anyFAST')).toBeInTheDocument()
  })

  it('shows advanced section with hosts file button', async () => {
    await renderSettings()

    expect(screen.getByText('高级')).toBeInTheDocument()
    expect(screen.getByText('Hosts 文件')).toBeInTheDocument()
    expect(screen.getByText('打开')).toBeInTheDocument()
  })

  it('shows network repair section', async () => {
    await renderSettings()

    expect(screen.getByText('网络修复')).toBeInTheDocument()
    expect(screen.getByText('清理当前进程代理')).toBeInTheDocument()
  })

  it('shows about section with version info', async () => {
    await renderSettings()

    expect(screen.getByText('关于')).toBeInTheDocument()
    expect(screen.getByText('当前版本')).toBeInTheDocument()
    expect(screen.getByText('检查更新')).toBeInTheDocument()
  })

  it('shows restore defaults button', async () => {
    await renderSettings()

    expect(screen.getByText('恢复默认值')).toBeInTheDocument()
  })

  it('calls set_autostart when autostart toggle is clicked', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'set_autostart') {
        return undefined
      }
      if (cmd === 'get_current_version') {
        return '2.6.3'
      }
      if (cmd === 'get_autostart') {
        return false
      }
      if (cmd === 'detect_system_proxy') {
        return 'http://127.0.0.1:7890'
      }
      if (cmd === 'get_proxy_env_snapshot') {
        return mockProxySnapshot
      }
      return undefined
    })

    await renderSettings()

    // Find the autostart toggle by its parent label text
    const autostartLabel = screen.getByText('开机自启动').closest('label')
    const toggle = autostartLabel?.querySelector('.rounded-full')

    if (toggle) {
      fireEvent.click(toggle)

      await waitFor(() => {
        expect(invoke).toHaveBeenCalledWith('set_autostart', { enabled: true })
      })
    }
  })

  it('calls open_hosts_file when hosts button is clicked', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'open_hosts_file') {
        return undefined
      }
      if (cmd === 'get_current_version') {
        return '2.6.3'
      }
      if (cmd === 'get_autostart') {
        return false
      }
      if (cmd === 'detect_system_proxy') {
        return 'http://127.0.0.1:7890'
      }
      if (cmd === 'get_proxy_env_snapshot') {
        return mockProxySnapshot
      }
      return undefined
    })

    await renderSettings()

    const openButton = screen.getByText('打开')
    fireEvent.click(openButton)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('open_hosts_file')
    })
  })

  it('calls updater check when check update button is clicked', async () => {
    const { check } = await import('@tauri-apps/plugin-updater')
    vi.mocked(check).mockResolvedValue(null)

    await renderSettings()

    const checkButton = screen.getByText('检查更新')
    fireEvent.click(checkButton)

    await waitFor(() => {
      expect(check).toHaveBeenCalled()
    })
  })

  it('clears process proxy env when clicking repair button', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'clear_process_proxy_env') {
        return {
          ...mockProxySnapshot,
          httpProxy: undefined,
          httpsProxy: undefined,
          allProxy: undefined,
          detectedSystemProxy: undefined,
        }
      }
      if (cmd === 'get_current_version') {
        return '2.6.3'
      }
      if (cmd === 'get_autostart') {
        return false
      }
      if (cmd === 'detect_system_proxy') {
        return 'http://127.0.0.1:7890'
      }
      if (cmd === 'get_proxy_env_snapshot') {
        return mockProxySnapshot
      }
      return undefined
    })

    await renderSettings()

    await waitFor(() => {
      expect(screen.getByRole('button', { name: '清理当前进程代理' })).not.toBeDisabled()
    })

    fireEvent.click(screen.getByRole('button', { name: '清理当前进程代理' }))

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('clear_process_proxy_env')
    })
  })

  it('restores defaults when restore button is clicked', async () => {
    const onEndpointsChange = vi.fn()
    const onConfigChange = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockImplementation(async (cmd: string) => {
      if (cmd === 'save_config') {
        return undefined
      }
      if (cmd === 'get_current_version') {
        return '2.6.3'
      }
      if (cmd === 'get_autostart') {
        return false
      }
      if (cmd === 'detect_system_proxy') {
        return 'http://127.0.0.1:7890'
      }
      if (cmd === 'get_proxy_env_snapshot') {
        return mockProxySnapshot
      }
      return undefined
    })

    await renderSettings({ ...defaultProps, onEndpointsChange, onConfigChange })

    const restoreButton = screen.getByText('恢复默认值')
    fireEvent.click(restoreButton)

    await waitFor(() => {
      expect(onEndpointsChange).toHaveBeenCalled()
      expect(invoke).toHaveBeenCalledWith('save_config', expect.any(Object))
    })
  })
})
