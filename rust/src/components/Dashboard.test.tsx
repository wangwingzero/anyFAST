import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Dashboard } from './Dashboard'
import type { Endpoint, EndpointResult, Progress } from '../types'

describe('Dashboard', () => {
  const mockEndpoints: Endpoint[] = [
    { name: 'Test 1', url: 'https://test1.com/v1', domain: 'test1.com', enabled: true },
    { name: 'Test 2', url: 'https://test2.com/v1', domain: 'test2.com', enabled: true },
  ]

  const mockResults: EndpointResult[] = [
    {
      endpoint: mockEndpoints[0],
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
      endpoint: mockEndpoints[1],
      ip: '2.3.4.5',
      latency: 150,
      ttfb: 150,
      success: true,
      original_ip: '6.7.8.9',
      original_latency: 300,
      speedup_percent: 50,
      use_original: false,
    },
  ]

  const mockProgress: Progress = {
    current: 0,
    total: 0,
    message: '就绪',
  }

  const defaultProps = {
    endpoints: mockEndpoints,
    results: [] as EndpointResult[],
    isRunning: false,
    progress: mockProgress,
    bindingCount: 0,
    testingDomains: new Set<string>(),
    onApply: vi.fn(),
    onApplyAll: vi.fn(),
    onUnbindAll: vi.fn(),
    onUnbindEndpoint: vi.fn(),
    onRetest: vi.fn(),
    onTestSingle: vi.fn(),
    onEndpointsChange: vi.fn(),
    onSaveConfig: vi.fn(),
  }

  beforeEach(() => {
    vi.clearAllMocks()
  })

  it('renders header correctly', () => {
    render(<Dashboard {...defaultProps} />)

    expect(screen.getByText('仪表盘')).toBeInTheDocument()
    expect(screen.getByText('测试中转站端点延迟')).toBeInTheDocument()
  })

  it('renders status cards with initial values', () => {
    render(<Dashboard {...defaultProps} />)

    expect(screen.getByText('已测')).toBeInTheDocument()
    expect(screen.getByText('可用')).toBeInTheDocument()
    expect(screen.getByText('绑定')).toBeInTheDocument()
  })

  it('shows empty state when no endpoints', () => {
    render(<Dashboard {...defaultProps} endpoints={[]} />)

    expect(screen.getByText('请先添加端点')).toBeInTheDocument()
  })

  // 三按钮模式测试
  it('shows 测速 button', () => {
    render(<Dashboard {...defaultProps} />)

    // 全局测速按钮 + 每行的测速按钮都有"测速"文本
    expect(screen.getAllByText('测速').length).toBeGreaterThanOrEqual(1)
  })

  it('shows 全部绑定 button', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    expect(screen.getByText('全部绑定')).toBeInTheDocument()
  })

  it('shows 全部解绑 button when bindings exist', () => {
    render(<Dashboard {...defaultProps} bindingCount={2} />)

    expect(screen.getByText('全部解绑')).toBeInTheDocument()
  })

  it('hides 全部解绑 button when no bindings', () => {
    render(<Dashboard {...defaultProps} bindingCount={0} />)

    expect(screen.queryByText('全部解绑')).not.toBeInTheDocument()
  })

  it('calls onRetest when clicking global 测速 button', () => {
    const onRetest = vi.fn()
    render(<Dashboard {...defaultProps} onRetest={onRetest} />)

    // 全局测速按钮是第一个包含"测速"文本的按钮
    const retestButtons = screen.getAllByText('测速')
    fireEvent.click(retestButtons[0])
    expect(onRetest).toHaveBeenCalledTimes(1)
  })

  it('calls onApplyAll when clicking 全部绑定 button', () => {
    const onApplyAll = vi.fn()
    render(<Dashboard {...defaultProps} results={mockResults} onApplyAll={onApplyAll} />)

    fireEvent.click(screen.getByText('全部绑定'))
    expect(onApplyAll).toHaveBeenCalledTimes(1)
  })

  it('calls onUnbindAll when clicking 全部解绑 button', () => {
    const onUnbindAll = vi.fn()
    render(<Dashboard {...defaultProps} bindingCount={2} onUnbindAll={onUnbindAll} />)

    fireEvent.click(screen.getByText('全部解绑'))
    expect(onUnbindAll).toHaveBeenCalledTimes(1)
  })

  it('disables 测速 button when no endpoints', () => {
    render(<Dashboard {...defaultProps} endpoints={[]} />)

    // 无端点时只有全局测速按钮
    const retestBtn = screen.getByText('测速').closest('button')
    expect(retestBtn).toBeDisabled()
  })

  it('disables 全部绑定 button when no available results', () => {
    render(<Dashboard {...defaultProps} results={[]} />)

    const applyAllBtn = screen.getByText('全部绑定').closest('button')
    expect(applyAllBtn).toBeDisabled()
  })

  it('renders results table with data', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    expect(screen.getAllByText('Test 1').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('Test 2').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('https://test1.com/v1').length).toBeGreaterThanOrEqual(1)
    expect(screen.getAllByText('https://test2.com/v1').length).toBeGreaterThanOrEqual(1)
    expect(screen.getByText('100ms')).toBeInTheDocument()
    expect(screen.getByText('150ms')).toBeInTheDocument()
  })

  it('calls onApply when clicking per-endpoint bind button', () => {
    const onApply = vi.fn()
    render(<Dashboard {...defaultProps} results={mockResults} onApply={onApply} />)

    // Per-endpoint bind buttons use Link icon with title "绑定到 hosts"
    const bindButtons = screen.getAllByTitle('绑定到 hosts')
    expect(bindButtons).toHaveLength(2)
    fireEvent.click(bindButtons[0])
    expect(onApply).toHaveBeenCalledWith(mockResults[0])
  })

  it('shows per-endpoint unbind buttons', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    const unbindButtons = screen.getAllByTitle('解绑 hosts')
    expect(unbindButtons).toHaveLength(2)
  })

  it('calls onUnbindEndpoint when clicking per-endpoint unbind button', () => {
    const onUnbindEndpoint = vi.fn()
    render(<Dashboard {...defaultProps} results={mockResults} onUnbindEndpoint={onUnbindEndpoint} />)

    const unbindButtons = screen.getAllByTitle('解绑 hosts')
    fireEvent.click(unbindButtons[0])
    expect(onUnbindEndpoint).toHaveBeenCalledWith('test1.com')
  })

  it('removes endpoint directly from results row', () => {
    const onEndpointsChange = vi.fn()
    const onSaveConfig = vi.fn()
    render(
      <Dashboard
        {...defaultProps}
        results={mockResults}
        onEndpointsChange={onEndpointsChange}
        onSaveConfig={onSaveConfig}
      />,
    )

    fireEvent.click(screen.getByLabelText('删除测速站点 Test 1'))

    const expected = [mockEndpoints[1]]
    expect(onEndpointsChange).toHaveBeenCalledWith(expected)
    expect(onSaveConfig).toHaveBeenCalledWith(expected)
  })

  it('shows speedup percentage for optimized results', () => {
    render(<Dashboard {...defaultProps} results={mockResults} />)

    const speedupBadges = screen.getAllByText(/↑ 50%/)
    expect(speedupBadges).toHaveLength(2)
  })

  it('shows failed result correctly', () => {
    const failedResult: EndpointResult = {
      endpoint: mockEndpoints[0],
      ip: '1.2.3.4',
      latency: 9999,
      ttfb: 9999,
      success: false,
      error: '连接超时',
      original_ip: '',
      original_latency: 0,
      speedup_percent: 0,
      use_original: false,
    }

    render(<Dashboard {...defaultProps} results={[failedResult]} />)

    expect(screen.getByText('连接超时')).toBeInTheDocument()
    expect(screen.queryByText('9999ms')).not.toBeInTheDocument()
    // No bind button for failed results
    expect(screen.queryByTitle('绑定到 hosts')).not.toBeInTheDocument()
  })

  it('updates binding count display', () => {
    render(<Dashboard {...defaultProps} bindingCount={5} />)

    expect(screen.getByText('5')).toBeInTheDocument()
  })
})
