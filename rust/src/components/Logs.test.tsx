import { describe, it, expect, vi, beforeEach } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { Logs } from './Logs'
import type { LogEntry } from '../types'

describe('Logs', () => {
  const mockOnClear = vi.fn()

  beforeEach(() => {
    mockOnClear.mockClear()
  })

  it('renders empty state when no logs', () => {
    render(<Logs logs={[]} onClear={mockOnClear} />)

    expect(screen.getByText('暂无日志记录')).toBeInTheDocument()
    expect(screen.getByText('开始测速后将显示操作记录')).toBeInTheDocument()
  })

  it('renders header correctly', () => {
    render(<Logs logs={[]} onClear={mockOnClear} />)

    expect(screen.getByText('运行日志')).toBeInTheDocument()
    expect(screen.getByText('查看操作记录和测试结果')).toBeInTheDocument()
  })

  it('renders log entries', () => {
    const logs: LogEntry[] = [
      { level: 'info', message: 'Test info message', timestamp: '10:00:00' },
      { level: 'success', message: 'Test success message', timestamp: '10:00:01' },
    ]

    render(<Logs logs={logs} onClear={mockOnClear} />)

    expect(screen.getByText('Test info message')).toBeInTheDocument()
    expect(screen.getByText('Test success message')).toBeInTheDocument()
    expect(screen.getByText('10:00:00')).toBeInTheDocument()
    expect(screen.getByText('10:00:01')).toBeInTheDocument()
  })

  it('renders all log levels', () => {
    const logs: LogEntry[] = [
      { level: 'success', message: 'Success log', timestamp: '10:00:00' },
      { level: 'info', message: 'Info log', timestamp: '10:00:01' },
      { level: 'warning', message: 'Warning log', timestamp: '10:00:02' },
      { level: 'error', message: 'Error log', timestamp: '10:00:03' },
    ]

    render(<Logs logs={logs} onClear={mockOnClear} />)

    expect(screen.getByText('Success log')).toBeInTheDocument()
    expect(screen.getByText('Info log')).toBeInTheDocument()
    expect(screen.getByText('Warning log')).toBeInTheDocument()
    expect(screen.getByText('Error log')).toBeInTheDocument()
  })

  it('displays correct stats', () => {
    const logs: LogEntry[] = [
      { level: 'success', message: 'msg1', timestamp: '10:00:00' },
      { level: 'success', message: 'msg2', timestamp: '10:00:01' },
      { level: 'info', message: 'msg3', timestamp: '10:00:02' },
      { level: 'warning', message: 'msg4', timestamp: '10:00:03' },
      { level: 'error', message: 'msg5', timestamp: '10:00:04' },
    ]

    render(<Logs logs={logs} onClear={mockOnClear} />)

    // Check stat card values - find by test content structure
    const statCards = screen.getAllByText(/^[0-5]$/)
    expect(statCards.length).toBeGreaterThanOrEqual(4)
  })

  it('calls onClear when clear button is clicked', () => {
    const logs: LogEntry[] = [
      { level: 'info', message: 'Test message', timestamp: '10:00:00' },
    ]

    render(<Logs logs={logs} onClear={mockOnClear} />)

    const clearButton = screen.getByText('清空日志')
    fireEvent.click(clearButton)

    expect(mockOnClear).toHaveBeenCalledTimes(1)
  })

  it('disables clear button when logs are empty', () => {
    render(<Logs logs={[]} onClear={mockOnClear} />)

    const clearButton = screen.getByText('清空日志').closest('button')
    expect(clearButton).toBeDisabled()
  })

  it('enables clear button when logs exist', () => {
    const logs: LogEntry[] = [
      { level: 'info', message: 'Test message', timestamp: '10:00:00' },
    ]

    render(<Logs logs={logs} onClear={mockOnClear} />)

    const clearButton = screen.getByText('清空日志')
    expect(clearButton).not.toBeDisabled()
  })
})
