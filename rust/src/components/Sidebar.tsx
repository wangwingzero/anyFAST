import { useState, useEffect } from 'react'
import { Activity, Settings, Zap, FileText, ShieldAlert, ShieldCheck, BarChart3 } from 'lucide-react'
import { invoke } from '@tauri-apps/api/core'

interface SidebarProps {
  currentView: 'dashboard' | 'settings' | 'logs' | 'history'
  onNavigate: (view: 'dashboard' | 'settings' | 'logs' | 'history') => void
}

export function Sidebar({ currentView, onNavigate }: SidebarProps) {
  const [isAdmin, setIsAdmin] = useState<boolean | null>(null)

  useEffect(() => {
    invoke<boolean>('check_admin').then(setIsAdmin).catch(() => setIsAdmin(false))
  }, [])

  const items = [
    { id: 'dashboard' as const, icon: Activity, label: '仪表盘' },
    { id: 'history' as const, icon: BarChart3, label: '历史统计' },
    { id: 'logs' as const, icon: FileText, label: '运行日志' },
    { id: 'settings' as const, icon: Settings, label: '设置' },
  ]

  return (
    <aside className="w-56 glass border-r border-apple-gray-200 flex flex-col">
      {/* Logo */}
      <div className="h-14 flex items-center px-5 border-b border-apple-gray-200">
        <Zap className="w-5 h-5 text-apple-blue mr-2" />
        <span className="font-semibold text-apple-gray-600">anyFAST</span>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-3">
        {items.map((item) => {
          const Icon = item.icon
          const isActive = currentView === item.id

          return (
            <button
              key={item.id}
              onClick={() => onNavigate(item.id)}
              className={`
                w-full flex items-center gap-3 px-3 py-2.5 rounded-apple text-sm font-medium
                transition-all duration-150 btn-press
                ${isActive
                  ? 'bg-apple-blue text-white shadow-apple'
                  : 'text-apple-gray-500 hover:bg-apple-gray-200/50'
                }
              `}
            >
              <Icon className="w-4.5 h-4.5" />
              {item.label}
            </button>
          )
        })}
      </nav>

      {/* Status */}
      <div className="p-4 border-t border-apple-gray-200">
        {isAdmin === null ? (
          <div className="flex items-center gap-2 text-xs text-apple-gray-400">
            <span className="w-2 h-2 rounded-full bg-apple-gray-300 animate-pulse" />
            检查权限...
          </div>
        ) : isAdmin ? (
          <div className="flex items-center gap-2 text-xs text-apple-green">
            <ShieldCheck className="w-4 h-4" />
            管理员模式
          </div>
        ) : (
          <div className="flex items-center gap-2 text-xs text-apple-orange">
            <ShieldAlert className="w-4 h-4" />
            <span>需要管理员权限</span>
          </div>
        )}
      </div>
    </aside>
  )
}
