/**
 * Property-Based Tests for ToggleButton Component
 * 
 * **Validates: Requirements 2.4, 2.5**
 * 
 * Property 1: 按钮状态与显示一致性
 * 对于任意工作状态（isWorking），切换按钮的显示文字和图标应与状态一致：
 * - 当 isWorking 为 false 时，显示"启动"文字
 * - 当 isWorking 为 true 时，显示"停止"文字
 */

import { describe, it, expect, vi } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'
import * as fc from 'fast-check'
import { ToggleButton } from './Dashboard'

describe('ToggleButton Property-Based Tests', () => {
  /**
   * Property 1: 按钮状态与显示一致性
   * 
   * **Validates: Requirements 2.4, 2.5**
   * 
   * Requirement 2.4: WHEN Toggle_Button 处于停止状态时，THE Dashboard SHALL 显示"启动"文字和启动图标
   * Requirement 2.5: WHEN Toggle_Button 处于启动状态时，THE Dashboard SHALL 显示"停止"文字和停止图标
   */
  it('Property 1: Button text and icon should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        // Generate random boolean values for isWorking state
        fc.boolean(),
        (isWorking: boolean) => {
          // Clean up any previous renders
          cleanup()
          
          // Render the ToggleButton with the generated isWorking state
          render(
            <ToggleButton
              isWorking={isWorking}
              isLoading={false}
              disabled={false}
              onClick={vi.fn()}
            />
          )
          
          // Get the button text element
          const buttonText = screen.getByTestId('toggle-button-text')
          
          if (isWorking) {
            // Requirement 2.5: When isWorking is true, should display "停止" text
            expect(buttonText).toHaveTextContent('停止')
            // Should display stop icon
            expect(screen.getByTestId('toggle-button-stop-icon')).toBeInTheDocument()
            // Should NOT display start icon
            expect(screen.queryByTestId('toggle-button-start-icon')).not.toBeInTheDocument()
          } else {
            // Requirement 2.4: When isWorking is false, should display "启动" text
            expect(buttonText).toHaveTextContent('启动')
            // Should display start icon
            expect(screen.getByTestId('toggle-button-start-icon')).toBeInTheDocument()
            // Should NOT display stop icon
            expect(screen.queryByTestId('toggle-button-stop-icon')).not.toBeInTheDocument()
          }
          
          return true
        }
      ),
      { numRuns: 100 } // Run at least 100 iterations as specified in the design
    )
  })

  /**
   * Property 1 (Extended): Button aria-label should be consistent with isWorking state
   * 
   * **Validates: Requirements 2.4, 2.5**
   * 
   * This extends Property 1 to also verify accessibility attributes are consistent
   */
  it('Property 1 (Extended): Button aria-label should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        (isWorking: boolean) => {
          cleanup()
          
          render(
            <ToggleButton
              isWorking={isWorking}
              isLoading={false}
              disabled={false}
              onClick={vi.fn()}
            />
          )
          
          const button = screen.getByTestId('toggle-button')
          const expectedLabel = isWorking ? '停止' : '启动'
          
          expect(button).toHaveAttribute('aria-label', expectedLabel)
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })

  /**
   * Property 1 (Extended): Button styling should be consistent with isWorking state
   * 
   * **Validates: Requirements 2.4, 2.5**
   * 
   * This extends Property 1 to verify visual styling is consistent:
   * - When isWorking is false (stopped): green button (启动)
   * - When isWorking is true (working): red button (停止)
   */
  it('Property 1 (Extended): Button styling should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        (isWorking: boolean) => {
          cleanup()
          
          render(
            <ToggleButton
              isWorking={isWorking}
              isLoading={false}
              disabled={false}
              onClick={vi.fn()}
            />
          )
          
          const button = screen.getByTestId('toggle-button')
          
          if (isWorking) {
            // Working state should have red styling (stop button)
            expect(button.className).toContain('bg-apple-red')
            expect(button.className).not.toContain('bg-apple-green')
          } else {
            // Stopped state should have green styling (start button)
            expect(button.className).toContain('bg-apple-green')
            expect(button.className).not.toContain('bg-apple-red')
          }
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })
})
