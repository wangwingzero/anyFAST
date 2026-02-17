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
  const mockEndpoints: Endpoint[] = [
    { name: 'Test 1', url: 'https://test1.com/v1', domain: 'test1.com', enabled: true },
    { name: 'Test 2', url: 'https://test2.com/v1', domain: 'test2.com', enabled: false },
  ]

  const mockConfig: AppConfig = {
    endpoints: mockEndpoints,
    autostart: false,
    preferred_ips: [],
  }

  const defaultProps = {
    config: mockConfig,
    onEndpointsChange: vi.fn(),
    onConfigChange: vi.fn(),
  }

  beforeEach(async () => {
    vi.clearAllMocks()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(undefined)
  })

  it('renders header correctly', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('设置')).toBeInTheDocument()
    expect(screen.getByText('配置运行参数')).toBeInTheDocument()
  })

  it('shows system section with autostart toggle', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('系统')).toBeInTheDocument()
    expect(screen.getByText('开机自启动')).toBeInTheDocument()
    expect(screen.getByText('系统启动时自动运行 anyFAST')).toBeInTheDocument()
  })

  it('shows advanced section with hosts file button', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('高级')).toBeInTheDocument()
    expect(screen.getByText('Hosts 文件')).toBeInTheDocument()
    expect(screen.getByText('打开')).toBeInTheDocument()
  })

  it('shows about section with version info', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('关于')).toBeInTheDocument()
    expect(screen.getByText('当前版本')).toBeInTheDocument()
    expect(screen.getByText('检查更新')).toBeInTheDocument()
  })

  it('shows restore defaults button', () => {
    render(<Settings {...defaultProps} />)

    expect(screen.getByText('恢复默认值')).toBeInTheDocument()
  })

  it('calls set_autostart when autostart toggle is clicked', async () => {
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(undefined)

    render(<Settings {...defaultProps} />)

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
    vi.mocked(invoke).mockResolvedValue(undefined)

    render(<Settings {...defaultProps} />)

    const openButton = screen.getByText('打开')
    fireEvent.click(openButton)

    await waitFor(() => {
      expect(invoke).toHaveBeenCalledWith('open_hosts_file')
    })
  })

  it('calls updater check when check update button is clicked', async () => {
    const { check } = await import('@tauri-apps/plugin-updater')
    vi.mocked(check).mockResolvedValue(null)

    render(<Settings {...defaultProps} />)

    const checkButton = screen.getByText('检查更新')
    fireEvent.click(checkButton)

    await waitFor(() => {
      expect(check).toHaveBeenCalled()
    })
  })

  it('restores defaults when restore button is clicked', async () => {
    const onEndpointsChange = vi.fn()
    const onConfigChange = vi.fn()
    const { invoke } = await import('@tauri-apps/api/core')
    vi.mocked(invoke).mockResolvedValue(undefined)

    render(<Settings {...defaultProps} onEndpointsChange={onEndpointsChange} onConfigChange={onConfigChange} />)

    const restoreButton = screen.getByText('恢复默认值')
    fireEvent.click(restoreButton)

    await waitFor(() => {
      expect(onEndpointsChange).toHaveBeenCalled()
      expect(invoke).toHaveBeenCalledWith('save_config', expect.any(Object))
    })
  })
})
