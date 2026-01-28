import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Sidebar } from './components/Sidebar'
import { Dashboard } from './components/Dashboard'
import { Settings } from './components/Settings'
import { Logs } from './components/Logs'
import { HistoryView } from './components/HistoryView'
import { ToastContainer, ToastData, ToastType } from './components'
import { Endpoint, EndpointResult, AppConfig, LogEntry, EndpointHealth } from './types'

type View = 'dashboard' | 'settings' | 'logs' | 'history'

let toastIdCounter = 0

// 检查是否是权限错误
const isPermissionError = (error: unknown): boolean => {
  const errorStr = String(error).toLowerCase()
  return errorStr.includes('permission denied') || 
         errorStr.includes('access denied') ||
         errorStr.includes('administrator')
}

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')
  const [endpoints, setEndpoints] = useState<Endpoint[]>([])
  const [results, setResults] = useState<EndpointResult[]>([])
  const [isRunning, setIsRunning] = useState(false)
  const [progress, setProgress] = useState({ current: 0, total: 0, message: '就绪' })
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [bindingCount, setBindingCount] = useState(0)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [toasts, setToasts] = useState<ToastData[]>([])
  const [showAdminDialog, setShowAdminDialog] = useState(false)
  const [healthStatus, setHealthStatus] = useState<EndpointHealth[]>([])

  const showToast = useCallback((type: ToastType, message: string) => {
    const id = ++toastIdCounter
    setToasts((prev) => [...prev, { id, type, message }])
  }, [])

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  // 处理权限错误，提示用户重启为管理员
  const handlePermissionError = useCallback((error: unknown) => {
    if (isPermissionError(error)) {
      setShowAdminDialog(true)
      return true
    }
    return false
  }, [])

  // 以管理员身份重启
  const restartAsAdmin = useCallback(async () => {
    try {
      await invoke('restart_as_admin')
    } catch {
      // 用户取消或出错
      setShowAdminDialog(false)
    }
  }, [])

  // 添加日志
  const addLog = useCallback((level: LogEntry['level'], message: string) => {
    const now = new Date()
    const timestamp = now.toLocaleTimeString('zh-CN', { hour12: false })
    setLogs((prev) => {
      const newLogs = [...prev, { level, message, timestamp }]
      // 限制最多 500 条
      if (newLogs.length > 500) {
        return newLogs.slice(-500)
      }
      return newLogs
    })
  }, [])

  // 清空日志
  const clearLogs = useCallback(() => {
    setLogs([])
    addLog('info', '日志已清空')
  }, [addLog])

  useEffect(() => {
    loadConfig()
    refreshBindingCount()

    // 监听健康检查结果事件
    const unlistenHealth = listen<{ endpoints_health: EndpointHealth[] }>('health-check-result', (event) => {
      setHealthStatus(event.payload.endpoints_health)
    })

    return () => {
      unlistenHealth.then(fn => fn())
    }
  }, [])

  const loadConfig = async () => {
    try {
      const cfg = await invoke<AppConfig>('get_config')
      setConfig(cfg)
      setEndpoints(cfg.endpoints)
      addLog('info', `已加载配置，${cfg.endpoints.length} 个端点`)
    } catch (e) {
      console.error('Failed to load config:', e)
      addLog('error', `加载配置失败: ${e}`)
    }
  }

  const refreshBindingCount = async () => {
    try {
      const count = await invoke<number>('get_binding_count')
      setBindingCount(count)
    } catch (e) {
      console.error('Failed to get binding count:', e)
    }
  }

  const startTest = async () => {
    const enabledCount = endpoints.filter((e) => e.enabled).length
    if (enabledCount === 0) {
      addLog('warning', '没有启用的端点，请先在设置中添加')
      return
    }

    setIsRunning(true)
    setResults([])
    setProgress({ current: 0, total: enabledCount, message: '正在测试...' })
    addLog('info', `开始测试 ${enabledCount} 个端点...`)
    addLog('info', `端点列表: ${endpoints.filter(e => e.enabled).map(e => e.name).join(', ')}`)

    const startTime = Date.now()

    try {
      addLog('info', '正在进行 DNS 解析和 HTTPS 测试...')
      const res = await invoke<EndpointResult[]>('start_speed_test')
      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1)
      setResults(res)

      const successCount = res.filter((r) => r.success).length
      setProgress({ current: res.length, total: res.length, message: '测试完成' })
      addLog('success', `测试完成: ${successCount}/${res.length} 个端点可用 (耗时 ${elapsed}s)`)

      // 记录每个端点的详细结果
      res.forEach((r, i) => {
        if (r.success) {
          const speedup = r.speedup_percent !== undefined && r.speedup_percent > 0
            ? ` (加速 ${r.speedup_percent.toFixed(0)}%)`
            : ''
          addLog('success', `[${i + 1}] ${r.endpoint.name}: ${r.latency.toFixed(0)}ms → ${r.ip}${speedup}`)
        } else {
          addLog('error', `[${i + 1}] ${r.endpoint.name}: ${r.error || '连接失败'}`)
        }
      })

      // 汇总信息
      if (successCount > 0) {
        const avgLatency = res.filter(r => r.success).reduce((sum, r) => sum + r.latency, 0) / successCount
        addLog('info', `平均延迟: ${avgLatency.toFixed(0)}ms`)
      }
    } catch (e) {
      const elapsed = ((Date.now() - startTime) / 1000).toFixed(1)
      console.error('Test failed:', e)
      setProgress({ current: 0, total: 0, message: `错误: ${e}` })
      addLog('error', `测试失败 (耗时 ${elapsed}s): ${e}`)
      addLog('warning', '请检查网络连接，或查看控制台获取详细错误信息')
    } finally {
      setIsRunning(false)
    }
  }

  const stopTest = async () => {
    await invoke('stop_speed_test')
    setIsRunning(false)
    addLog('warning', '测试已取消')
  }

  const applyEndpoint = async (result: EndpointResult) => {
    try {
      await invoke('apply_endpoint', { domain: result.endpoint.domain, ip: result.ip })
      await refreshBindingCount()
      setProgress({ ...progress, message: `已绑定: ${result.endpoint.domain} → ${result.ip}` })
      addLog('success', `已绑定: ${result.endpoint.domain} → ${result.ip}`)
      showToast('success', `已绑定 ${result.endpoint.name}`)
    } catch (e) {
      console.error('Apply failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', `绑定失败: 需要管理员权限`)
      } else {
        setProgress({ ...progress, message: `绑定失败: ${e}` })
        addLog('error', `绑定失败: ${e}`)
        showToast('error', `绑定失败: ${e}`)
      }
    }
  }

  const applyAll = async () => {
    try {
      const count = await invoke<number>('apply_all_endpoints')
      await refreshBindingCount()
      setProgress({ ...progress, message: `已绑定 ${count} 个端点` })
      addLog('success', `一键应用完成: 已绑定 ${count} 个端点`)
      showToast('success', `已成功绑定 ${count} 个端点`)
    } catch (e) {
      console.error('Apply all failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', `一键应用失败: 需要管理员权限`)
      } else {
        setProgress({ ...progress, message: `绑定失败: ${e}` })
        addLog('error', `一键应用失败: ${e}`)
        showToast('error', `一键应用失败: ${e}`)
      }
    }
  }

  const clearBindings = async () => {
    try {
      const count = await invoke<number>('clear_all_bindings')
      await refreshBindingCount()
      setProgress({ ...progress, message: `已清除 ${count} 个绑定` })
      addLog('info', `已清除 ${count} 个绑定`)
      showToast('info', `已清除 ${count} 个绑定`)
    } catch (e) {
      console.error('Clear failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', `清除绑定失败: 需要管理员权限`)
      } else {
        setProgress({ ...progress, message: `清除失败: ${e}` })
        addLog('error', `清除绑定失败: ${e}`)
        showToast('error', `清除失败: ${e}`)
      }
    }
  }

  // 保存配置（用于仪表盘端点管理）
  const saveConfigWithEndpoints = async (newEndpoints: Endpoint[]) => {
    if (!config) return
    const newConfig = { ...config, endpoints: newEndpoints }
    try {
      await invoke('save_config', { config: newConfig })
      setConfig(newConfig)
      await refreshBindingCount()
      addLog('info', '端点配置已保存')
    } catch (e) {
      console.error('Save config failed:', e)
      addLog('error', `保存配置失败: ${e}`)
      showToast('error', `保存失败: ${e}`)
    }
  }

  return (
    <div className="flex h-screen bg-[#F5F5F7]">
      {/* Toast Notifications */}
      <ToastContainer toasts={toasts} onClose={removeToast} />

      {/* Admin Permission Dialog */}
      {showAdminDialog && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
          <div className="bg-white rounded-2xl shadow-2xl p-6 max-w-md mx-4 animate-in fade-in zoom-in duration-200">
            <div className="flex items-center gap-3 mb-4">
              <div className="w-12 h-12 rounded-full bg-orange-100 flex items-center justify-center">
                <svg className="w-6 h-6 text-orange-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
                </svg>
              </div>
              <div>
                <h3 className="text-lg font-semibold text-gray-900">需要管理员权限</h3>
                <p className="text-sm text-gray-500">Service 连接失败，需要提升权限</p>
              </div>
            </div>
            <p className="text-sm text-gray-600 mb-6">
              修改 hosts 文件需要管理员权限。是否以管理员身份重新启动应用？
            </p>
            <div className="flex gap-3">
              <button
                onClick={() => setShowAdminDialog(false)}
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
              >
                取消
              </button>
              <button
                onClick={restartAsAdmin}
                className="flex-1 px-4 py-2 text-sm font-medium text-white bg-orange-500 rounded-xl hover:bg-orange-600 transition-colors"
              >
                以管理员身份重启
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Sidebar */}
      <Sidebar currentView={currentView} onNavigate={setCurrentView} />

      {/* Main Content */}
      <main className="flex-1 overflow-auto min-w-0">
        {currentView === 'dashboard' && (
          <Dashboard
            endpoints={endpoints}
            results={results}
            isRunning={isRunning}
            progress={progress}
            bindingCount={bindingCount}
            healthStatus={healthStatus}
            onStart={startTest}
            onStop={stopTest}
            onApply={applyEndpoint}
            onApplyAll={applyAll}
            onClearBindings={clearBindings}
            onEndpointsChange={setEndpoints}
            onSaveConfig={saveConfigWithEndpoints}
          />
        )}
        {currentView === 'history' && <HistoryView />}
        {currentView === 'logs' && (
          <Logs logs={logs} onClear={clearLogs} />
        )}
        {currentView === 'settings' && (
          <Settings
            endpoints={endpoints}
            config={config}
            onEndpointsChange={setEndpoints}
            onConfigChange={setConfig}
          />
        )}
      </main>
    </div>
  )
}

export default App
