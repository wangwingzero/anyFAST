import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Sidebar } from './components/Sidebar'
import { Dashboard } from './components/Dashboard'
import { Settings } from './components/Settings'
import { Logs } from './components/Logs'
import { HistoryView } from './components/HistoryView'
import { ToastContainer, ToastData, ToastType } from './components'
import { Endpoint, EndpointResult, AppConfig, LogEntry } from './types'

type View = 'dashboard' | 'settings' | 'logs' | 'history'

let toastIdCounter = 0

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

  const showToast = useCallback((type: ToastType, message: string) => {
    const id = ++toastIdCounter
    setToasts((prev) => [...prev, { id, type, message }])
  }, [])

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
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
      setProgress({ ...progress, message: `绑定失败: ${e}` })
      addLog('error', `绑定失败: ${e}`)
      showToast('error', `绑定失败: ${e}`)
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
      setProgress({ ...progress, message: `绑定失败: ${e}` })
      addLog('error', `一键应用失败: ${e}`)
      showToast('error', `一键应用失败: ${e}`)
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
      setProgress({ ...progress, message: `清除失败: ${e}` })
      addLog('error', `清除绑定失败: ${e}`)
      showToast('error', `清除失败: ${e}`)
    }
  }

  return (
    <div className="flex h-screen bg-apple-gray-100">
      {/* Toast Notifications */}
      <ToastContainer toasts={toasts} onClose={removeToast} />

      {/* Sidebar */}
      <Sidebar currentView={currentView} onNavigate={setCurrentView} />

      {/* Main Content */}
      <main className="flex-1 overflow-hidden">
        {currentView === 'dashboard' && (
          <Dashboard
            endpoints={endpoints}
            results={results}
            isRunning={isRunning}
            progress={progress}
            bindingCount={bindingCount}
            onStart={startTest}
            onStop={stopTest}
            onApply={applyEndpoint}
            onApplyAll={applyAll}
            onClearBindings={clearBindings}
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
