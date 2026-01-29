import { describe, it, expect, vi } from 'vitest'
import { render, screen, fireEvent } from '@testing-library/react'
import { ToggleButton } from './Dashboard'

describe('ToggleButton', () => {
  const defaultProps = {
    isWorking: false,
    isLoading: false,
    disabled: false,
    onClick: vi.fn(),
  }

  // Requirement 2.4: WHEN Toggle_Button 处于停止状态时，THE Dashboard SHALL 显示"启动"文字和启动图标
  it('shows "启动" text when isWorking is false (Requirement 2.4)', () => {
    render(<ToggleButton {...defaultProps} isWorking={false} />)
    
    expect(screen.getByTestId('toggle-button-text')).toHaveTextContent('启动')
    expect(screen.getByTestId('toggle-button-start-icon')).toBeInTheDocument()
  })

  // Requirement 2.5: WHEN Toggle_Button 处于启动状态时，THE Dashboard SHALL 显示"停止"文字和停止图标
  it('shows "停止" text when isWorking is true (Requirement 2.5)', () => {
    render(<ToggleButton {...defaultProps} isWorking={true} />)
    
    expect(screen.getByTestId('toggle-button-text')).toHaveTextContent('停止')
    expect(screen.getByTestId('toggle-button-stop-icon')).toBeInTheDocument()
  })

  it('shows loading spinner when isLoading is true', () => {
    render(<ToggleButton {...defaultProps} isLoading={true} />)
    
    expect(screen.getByTestId('toggle-button-loading')).toBeInTheDocument()
    // Icon should not be visible when loading
    expect(screen.queryByTestId('toggle-button-start-icon')).not.toBeInTheDocument()
  })

  it('is disabled when disabled prop is true', () => {
    render(<ToggleButton {...defaultProps} disabled={true} />)
    
    expect(screen.getByTestId('toggle-button')).toBeDisabled()
  })

  it('is disabled when isLoading is true', () => {
    render(<ToggleButton {...defaultProps} isLoading={true} />)
    
    expect(screen.getByTestId('toggle-button')).toBeDisabled()
  })

  it('calls onClick when clicked', () => {
    const onClick = vi.fn()
    render(<ToggleButton {...defaultProps} onClick={onClick} />)
    
    fireEvent.click(screen.getByTestId('toggle-button'))
    expect(onClick).toHaveBeenCalledTimes(1)
  })

  it('does not call onClick when disabled', () => {
    const onClick = vi.fn()
    render(<ToggleButton {...defaultProps} disabled={true} onClick={onClick} />)
    
    fireEvent.click(screen.getByTestId('toggle-button'))
    expect(onClick).not.toHaveBeenCalled()
  })

  it('has green styling when isWorking is false', () => {
    render(<ToggleButton {...defaultProps} isWorking={false} />)
    
    const button = screen.getByTestId('toggle-button')
    expect(button.className).toContain('bg-apple-green')
  })

  it('has red styling when isWorking is true', () => {
    render(<ToggleButton {...defaultProps} isWorking={true} />)
    
    const button = screen.getByTestId('toggle-button')
    expect(button.className).toContain('bg-apple-red')
  })

  // Requirement 5.2: WHEN System 处于工作状态时，THE Toggle_Button SHALL 显示醒目的活跃状态样式
  it('has active animation class when isWorking is true (Requirement 5.2)', () => {
    render(<ToggleButton {...defaultProps} isWorking={true} />)
    
    const button = screen.getByTestId('toggle-button')
    expect(button.className).toContain('toggle-button-active')
  })

  it('does not have active animation class when isWorking is false', () => {
    render(<ToggleButton {...defaultProps} isWorking={false} />)
    
    const button = screen.getByTestId('toggle-button')
    expect(button.className).not.toContain('toggle-button-active')
  })

  it('has correct aria-label for accessibility', () => {
    const { rerender } = render(<ToggleButton {...defaultProps} isWorking={false} />)
    expect(screen.getByTestId('toggle-button')).toHaveAttribute('aria-label', '启动')
    
    rerender(<ToggleButton {...defaultProps} isWorking={true} />)
    expect(screen.getByTestId('toggle-button')).toHaveAttribute('aria-label', '停止')
  })
})
