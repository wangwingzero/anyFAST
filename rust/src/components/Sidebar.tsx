import { useState, useEffect, useCallback } from 'react'
import { Activity, Settings, FileText, ShieldAlert, ShieldCheck, BarChart3, Server, RotateCcw, RefreshCw } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'
import { PermissionStatus } from '../types'

interface SidebarProps {
  currentView: 'dashboard' | 'settings' | 'logs' | 'history'
  onNavigate: (view: 'dashboard' | 'settings' | 'logs' | 'history') => void
}

export function Sidebar({ currentView, onNavigate }: SidebarProps) {
  const [permissionStatus, setPermissionStatus] = useState<PermissionStatus | null>(null)
  const [isRestarting, setIsRestarting] = useState(false)
  const [isRefreshing, setIsRefreshing] = useState(false)

  const refreshPermissionStatus = useCallback(async () => {
    setIsRefreshing(true)
    try {
      // 先刷新后端的 Service 状态缓存
      await invoke('refresh_service_status')
      // 然后获取最新的权限状态
      const status = await invoke<PermissionStatus>('get_permission_status')
      setPermissionStatus(status)
    } catch {
      setPermissionStatus({ hasPermission: false, isUsingService: false })
    } finally {
      setIsRefreshing(false)
    }
  }, [])

  useEffect(() => {
    refreshPermissionStatus()
  }, [])

  const handleRestartAsAdmin = async () => {
    setIsRestarting(true)
    try {
      await invoke('restart_as_admin')
    } catch {
      // User cancelled or error
      setIsRestarting(false)
    }
  }

  const items = [
    { id: 'dashboard' as const, icon: Activity, label: '仪表盘' },
    { id: 'history' as const, icon: BarChart3, label: '历史统计' },
    { id: 'logs' as const, icon: FileText, label: '运行日志' },
    { id: 'settings' as const, icon: Settings, label: '设置' },
  ]

  return (
    <aside className="w-16 lg:w-56 h-full bg-white/80 backdrop-blur-xl border-r border-gray-200/50 flex flex-col transition-[width] duration-300 ease-out">
      {/* Navigation */}
      <nav className="flex-1 p-2 lg:p-3">
        {items.map((item) => {
          const Icon = item.icon
          const isActive = currentView === item.id

          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              title={item.label}
              className={`
                w-full flex items-center justify-center lg:justify-start gap-0 lg:gap-3
                px-2 lg:px-3 py-2.5 mb-1 rounded-xl text-sm font-medium
                transition-all duration-200 btn-press
                ${isActive
                  ? 'bg-apple-blue text-white shadow-lg shadow-apple-blue/20'
                  : 'text-apple-gray-500 hover:bg-apple-gray-100'
                }
              `}
            >
              <Icon className="w-5 h-5 flex-shrink-0" aria-hidden="true" />
              <span className="hidden lg:inline">{item.label}</span>
            </button>
          )
        })}
      </nav>

      {/* Status */}
      <div className="p-2 lg:p-4 border-t border-apple-gray-200">
        {permissionStatus === null ? (
          // Loading skeleton
          <div className="flex items-center justify-center lg:justify-start gap-2 text-xs text-apple-gray-400">
            <span className="w-4 h-4 rounded bg-apple-gray-200 animate-pulse flex-shrink-0" />
            <span className="hidden lg:block w-20 h-3 rounded bg-apple-gray-200 animate-pulse" />
          </div>
        ) : permissionStatus.hasPermission ? (
          permissionStatus.isUsingService ? (
            <div className="flex items-center justify-center lg:justify-start gap-2 text-xs text-blue-600" title="Service 模式 - 点击刷新状态">
              <Server className="w-4 h-4 flex-shrink-0" aria-hidden="true" />
              <span className="hidden lg:inline">Service 模式</span>
              <button
                onClick={refreshPermissionStatus}
                disabled={isRefreshing}
                className="ml-auto p-1 hover:bg-blue-100 rounded transition-colors"
                title="刷新状态"
              >
                <RefreshCw className={`w-3 h-3 ${isRefreshing ? 'animate-spin' : ''}`} />
              </button>
            </div>
          ) : (
            <div className="flex items-center justify-center lg:justify-start gap-2 text-xs text-apple-green" title="管理员模式">
              <ShieldCheck className="w-4 h-4 flex-shrink-0" aria-hidden="true" />
              <span className="hidden lg:inline">管理员模式</span>
            </div>
          )
        ) : (
          <div className="space-y-2">
            <div className="flex items-center justify-center lg:justify-start gap-2 text-xs text-apple-orange" title="需要管理员权限">
              <ShieldAlert className="w-4 h-4 flex-shrink-0" aria-hidden="true" />
              <span className="hidden lg:inline">需要管理员权限</span>
            </div>
            <button
              onClick={handleRestartAsAdmin}
              disabled={isRestarting}
              title={isRestarting ? '正在重启...' : '以管理员身份重启'}
              className="w-full flex items-center justify-center gap-1.5 px-2 py-1.5
                text-xs font-medium text-white bg-apple-orange rounded-lg
                hover:bg-orange-600 transition-colors disabled:opacity-50"
            >
              <RotateCcw className={`w-3 h-3 flex-shrink-0 ${isRestarting ? 'animate-spin' : ''}`} aria-hidden="true" />
              <span className="hidden lg:inline">{isRestarting ? '正在重启...' : '以管理员身份重启'}</span>
            </button>
          </div>
        )}
      </div>
    </aside>
  )
}
