/**
 * Property-Based Tests for WorkingIndicator Component
 * 
 * **Validates: Requirements 5.1, 5.2, 5.3**
 * 
 * Property 2: 工作状态与视觉样式一致性
 * 对于任意工作状态（isWorking），视觉指示器的样式应与状态一致：
 * - 当 isWorking 为 true 时，应用脉冲动画 CSS 类
 * - 当 isWorking 为 false 时，移除脉冲动画 CSS 类
 */

import { describe, it, expect } from 'vitest'
import { render, screen, cleanup } from '@testing-library/react'
import * as fc from 'fast-check'
import { WorkingIndicator } from './Dashboard'

describe('WorkingIndicator Property-Based Tests', () => {
  /**
   * Property 2: 工作状态与视觉样式一致性
   * 
   * **Validates: Requirements 5.1, 5.2, 5.3**
   * 
   * Requirement 5.1: WHEN System 处于工作状态时，THE Working_Indicator SHALL 显示脉冲动画效果
   * Requirement 5.2: WHEN System 处于工作状态时，THE Toggle_Button SHALL 显示醒目的活跃状态样式
   * Requirement 5.3: WHEN System 处于停止状态时，THE Working_Indicator SHALL 停止动画并显示静态样式
   */
  it('Property 2: Pulse animation CSS class should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        // Generate random boolean values for isWorking state
        fc.boolean(),
        // Generate random non-negative integers for bindingCount
        fc.nat({ max: 100 }),
        (isWorking: boolean, bindingCount: number) => {
          // Clean up any previous renders
          cleanup()
          
          // Render the WorkingIndicator with the generated state
          render(
            <WorkingIndicator
              isWorking={isWorking}
              bindingCount={bindingCount}
            />
          )
          
          // Get the indicator dot element
          const dot = screen.getByTestId('working-indicator-dot')
          
          if (isWorking) {
            // Requirement 5.1: When isWorking is true, should apply pulse animation CSS class
            expect(dot).toHaveClass('working-indicator-pulse')
            expect(dot).toHaveClass('bg-apple-green')
          } else {
            // Requirement 5.3: When isWorking is false, should NOT have pulse animation CSS class
            expect(dot).not.toHaveClass('working-indicator-pulse')
            expect(dot).toHaveClass('bg-apple-gray-400')
          }
          
          return true
        }
      ),
      { numRuns: 100 } // Run at least 100 iterations as specified in the design
    )
  })

  /**
   * Property 2 (Extended): Container styling should be consistent with isWorking state
   * 
   * **Validates: Requirements 5.1, 5.2, 5.3**
   * 
   * This extends Property 2 to verify container background and border styling:
   * - When isWorking is true: green-tinted background and border
   * - When isWorking is false: gray background and border
   */
  it('Property 2 (Extended): Container styling should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        fc.nat({ max: 100 }),
        (isWorking: boolean, bindingCount: number) => {
          cleanup()
          
          render(
            <WorkingIndicator
              isWorking={isWorking}
              bindingCount={bindingCount}
            />
          )
          
          const indicator = screen.getByTestId('working-indicator')
          
          if (isWorking) {
            // Working state should have green-tinted styling
            expect(indicator).toHaveClass('bg-apple-green/10')
            expect(indicator).toHaveClass('border-apple-green/30')
          } else {
            // Stopped state should have gray styling
            expect(indicator).toHaveClass('bg-apple-gray-100')
            expect(indicator).toHaveClass('border-apple-gray-200')
          }
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })

  /**
   * Property 2 (Extended): Text styling should be consistent with isWorking state
   * 
   * **Validates: Requirements 5.1, 5.2, 5.3**
   * 
   * This extends Property 2 to verify text color styling:
   * - When isWorking is true: green text color
   * - When isWorking is false: gray text color
   */
  it('Property 2 (Extended): Text styling should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        fc.nat({ max: 100 }),
        (isWorking: boolean, bindingCount: number) => {
          cleanup()
          
          render(
            <WorkingIndicator
              isWorking={isWorking}
              bindingCount={bindingCount}
            />
          )
          
          const text = screen.getByTestId('working-indicator-text')
          
          if (isWorking) {
            // Working state should have green text
            expect(text).toHaveClass('text-apple-green')
            expect(text).toHaveTextContent('工作中')
          } else {
            // Stopped state should have gray text
            expect(text).toHaveClass('text-apple-gray-500')
            expect(text).toHaveTextContent('已停止')
          }
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })

  /**
   * Property 2 (Extended): Icon styling should be consistent with isWorking state
   * 
   * **Validates: Requirements 5.1, 5.2, 5.3**
   * 
   * This extends Property 2 to verify icon color styling:
   * - When isWorking is true: green icon color
   * - When isWorking is false: gray icon color
   */
  it('Property 2 (Extended): Icon styling should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        fc.nat({ max: 100 }),
        (isWorking: boolean, bindingCount: number) => {
          cleanup()
          
          render(
            <WorkingIndicator
              isWorking={isWorking}
              bindingCount={bindingCount}
            />
          )
          
          const icon = screen.getByTestId('working-indicator-icon')
          
          if (isWorking) {
            // Working state should have green icon
            expect(icon).toHaveClass('text-apple-green')
          } else {
            // Stopped state should have gray icon
            expect(icon).toHaveClass('text-apple-gray-400')
          }
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })

  /**
   * Property 2 (Extended): Aria-label should be consistent with isWorking state
   * 
   * **Validates: Requirements 5.1, 5.2, 5.3**
   * 
   * This extends Property 2 to verify accessibility attributes are consistent
   */
  it('Property 2 (Extended): Aria-label should be consistent with isWorking state', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        fc.nat({ max: 100 }),
        (isWorking: boolean, bindingCount: number) => {
          cleanup()
          
          render(
            <WorkingIndicator
              isWorking={isWorking}
              bindingCount={bindingCount}
            />
          )
          
          const indicator = screen.getByTestId('working-indicator')
          const expectedLabel = isWorking ? '工作状态: 工作中' : '工作状态: 已停止'
          
          expect(indicator).toHaveAttribute('aria-label', expectedLabel)
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })

  /**
   * Property 2 (Extended): Binding count display should be consistent with bindingCount value
   * 
   * **Validates: Requirements 5.1, 5.2, 5.3**
   * 
   * This extends Property 2 to verify binding count display:
   * - When bindingCount > 0: should display binding count
   * - When bindingCount === 0: should NOT display binding count
   */
  it('Property 2 (Extended): Binding count display should be consistent with bindingCount value', () => {
    fc.assert(
      fc.property(
        fc.boolean(),
        fc.nat({ max: 100 }),
        (isWorking: boolean, bindingCount: number) => {
          cleanup()
          
          render(
            <WorkingIndicator
              isWorking={isWorking}
              bindingCount={bindingCount}
            />
          )
          
          const bindingCountElement = screen.queryByTestId('working-indicator-binding-count')
          
          if (bindingCount > 0) {
            // Should display binding count when > 0
            expect(bindingCountElement).toBeInTheDocument()
            expect(bindingCountElement).toHaveTextContent(`(${bindingCount} 绑定)`)
          } else {
            // Should NOT display binding count when === 0
            expect(bindingCountElement).not.toBeInTheDocument()
          }
          
          return true
        }
      ),
      { numRuns: 100 }
    )
  })
})
