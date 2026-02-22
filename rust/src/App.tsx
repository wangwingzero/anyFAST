import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { Sidebar } from './components/Sidebar'
import { Dashboard } from './components/Dashboard'
import { Settings } from './components/Settings'
import { Logs } from './components/Logs'
import { HistoryView } from './components/HistoryView'
import { ToastContainer, ToastData, ToastType } from './components'
import { Endpoint, EndpointResult, AppConfig, LogEntry, OptimizationEvent, TestProgressEvent } from './types'

type View = 'dashboard' | 'settings' | 'logs' | 'history'

let toastIdCounter = 0

// 检测操作系统
const isMacOS = navigator.userAgent.includes('Mac')

// 检查是否是权限错误
const isPermissionError = (error: unknown): boolean => {
  const errorStr = String(error).toLowerCase()
  return errorStr.includes('permission denied') ||
         errorStr.includes('access denied') ||
         errorStr.includes('administrator') ||
         errorStr.includes('拒绝访问') ||
         errorStr.includes('os error 5')
}

const sleepMs = (ms: number) => new Promise(resolve => setTimeout(resolve, ms))

function App() {
  const [currentView, setCurrentView] = useState<View>('dashboard')
  const [endpoints, setEndpoints] = useState<Endpoint[]>([])
  const [results, setResults] = useState<EndpointResult[]>([])
  const [isRunning, setIsRunning] = useState(false)
  const [, setProgress] = useState({ current: 0, total: 0, message: '就绪' })
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [bindingCount, setBindingCount] = useState(0)
  const [logs, setLogs] = useState<LogEntry[]>([])
  const [toasts, setToasts] = useState<ToastData[]>([])
  const [showAdminDialog, setShowAdminDialog] = useState(false)
  const [userDeclinedAdmin, setUserDeclinedAdmin] = useState(false)
  const [, setHasPermission] = useState<boolean | null>(null)
  const [isInstallingHelper, setIsInstallingHelper] = useState(false)
  const [isInstallingService, setIsInstallingService] = useState(false)
  const [isRunningAsAdmin, setIsRunningAsAdmin] = useState(false)
  const [hasBundledHelper, setHasBundledHelper] = useState(false)
  const [sidebarRefreshTrigger, setSidebarRefreshTrigger] = useState(0)
  const [testingDomains, setTestingDomains] = useState<Set<string>>(new Set())
  const [isOptimizing, setIsOptimizing] = useState(false)

  const showToast = useCallback((type: ToastType, message: string) => {
    const id = ++toastIdCounter
    setToasts((prev) => [...prev, { id, type, message }])
  }, [])

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  const addLog = useCallback((level: LogEntry['level'], message: string) => {
    const now = new Date()
    const timestamp = now.toLocaleTimeString('zh-CN', { hour12: false })
    setLogs((prev) => {
      const newLogs = [...prev, { level, message, timestamp }]
      if (newLogs.length > 500) {
        return newLogs.slice(-500)
      }
      return newLogs
    })
  }, [])

  const handlePermissionError = useCallback((error: unknown) => {
    if (isPermissionError(error)) {
      if (!userDeclinedAdmin) {
        setShowAdminDialog(true)
      }
      return true
    }
    return false
  }, [userDeclinedAdmin])

  const restartAsAdmin = useCallback(async () => {
    try {
      await invoke('restart_as_admin')
    } catch {
      setShowAdminDialog(false)
    }
  }, [])

  const installMacOSHelper = async () => {
    try {
      setIsInstallingHelper(true)
      addLog('info', '正在安装 macOS Helper...')
      const result = await invoke<boolean>('install_macos_helper')

      if (result) {
        addLog('success', 'Helper 安装成功！')
        showToast('success', '安装成功')
        setShowAdminDialog(false)
        setHasPermission(true)
        setSidebarRefreshTrigger(prev => prev + 1)
      } else {
        addLog('warning', '未找到内置 Helper，请从 GitHub 下载')
        showToast('warning', '请从 GitHub Release 下载 Helper')
      }
    } catch (e) {
      const errorStr = String(e)
      if (errorStr.includes('取消')) {
        addLog('info', '用户取消了安装')
      } else {
        addLog('error', `安装失败: ${e}`)
        showToast('error', `安装失败: ${e}`)
      }
    } finally {
      setIsInstallingHelper(false)
    }
  }

  const installWindowsService = async () => {
    try {
      setIsInstallingService(true)
      addLog('info', '正在安装 anyFAST Service...')
      const result = await invoke<string>('install_and_start_service')
      addLog('success', `Service 安装成功: ${result}`)
      showToast('success', 'Service 安装并启动成功')

      // Refresh and check
      await invoke('refresh_service_status')
      const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
      if (status.hasPermission) {
        setShowAdminDialog(false)
        setHasPermission(true)
        setSidebarRefreshTrigger(prev => prev + 1)
      }
    } catch (e) {
      const errorStr = String(e)
      addLog('error', `Service 安装失败: ${errorStr}`)
      showToast('error', `安装失败: ${errorStr}`)
    } finally {
      setIsInstallingService(false)
    }
  }

  const declineAdmin = useCallback(() => {
    setUserDeclinedAdmin(true)
    setShowAdminDialog(false)
    addLog('info', '已暂不授权管理员权限，部分功能可能受限')
  }, [addLog])

  const checkPermission = useCallback(async (): Promise<boolean> => {
    try {
      const maxAttempts = isMacOS ? 1 : 5
      let lastStatus: { hasPermission: boolean; isUsingService: boolean } = { hasPermission: false, isUsingService: false }

      for (let attempt = 1; attempt <= maxAttempts; attempt++) {
        if (!isMacOS) {
          try {
            await invoke('refresh_service_status')
          } catch {
            // ignore
          }
        }

        const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
        lastStatus = status

        if (status.hasPermission) {
          setHasPermission(true)
          if (status.isUsingService) {
            addLog('info', isMacOS ? '已连接到 anyFAST Helper' : '已连接到 anyFAST Service')
          } else {
            addLog('info', '以管理员身份运行')
          }
          return true
        }

        if (attempt < maxAttempts && !isMacOS) {
          await sleepMs(400 * attempt)
        }
      }

      // Windows: check if running as admin (for showing install-service option)
      if (!isMacOS) {
        try {
          const adminStatus = await invoke<boolean>('check_admin')
          setIsRunningAsAdmin(adminStatus)

          // Auto-try installing service if running as admin but service not available
          if (adminStatus && !lastStatus.isUsingService) {
            addLog('info', '以管理员身份运行但 Service 未安装，正在自动安装...')
            try {
              await invoke<string>('install_and_start_service')
              await sleepMs(500)
              await invoke('refresh_service_status')
              const newStatus = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
              if (newStatus.hasPermission) {
                setHasPermission(true)
                addLog('success', 'Service 自动安装并启动成功')
                setSidebarRefreshTrigger(prev => prev + 1)
                return true
              }
            } catch (e) {
              addLog('warning', `Service 自动安装失败: ${e}`)
            }
          }
        } catch {
          // ignore
        }
      }

      setHasPermission(false)
      return lastStatus.hasPermission
    } catch (e) {
      console.error('Failed to check permission:', e)
      setHasPermission(false)
      return false
    }
  }, [addLog])

  const checkBundledHelper = useCallback(async () => {
    if (isMacOS) {
      try {
        const has = await invoke<boolean>('has_bundled_helper')
        setHasBundledHelper(has)
      } catch {
        setHasBundledHelper(false)
      }
    }
  }, [])

  const clearLogs = useCallback(() => {
    setLogs([])
    addLog('info', '日志已清空')
  }, [addLog])

  const refreshBindingCount = useCallback(async () => {
    try {
      const count = await invoke<number>('get_binding_count')
      setBindingCount(count)
    } catch (e) {
      console.error('Failed to get binding count:', e)
    }
  }, [])

  useEffect(() => {
    initializeApp()
  }, [])

  // 监听持续优化事件
  useEffect(() => {
    const unlisten = listen<OptimizationEvent>('optimization-event', (event) => {
      const data = event.payload
      switch (data.eventType) {
        case 'started':
          setIsOptimizing(true)
          addLog('info', data.message)
          break
        case 'stopped':
          setIsOptimizing(false)
          addLog('info', data.message)
          break
        case 'auto_switch':
          showToast('success', data.message)
          addLog('success', data.message)
          // 刷新 results 和绑定计数
          invoke<EndpointResult[]>('get_current_results')
            .then(setResults)
            .catch(console.error)
          refreshBindingCount()
          break
        case 'check_complete':
          addLog('info', data.message)
          break
      }
    })
    return () => { unlisten.then(fn => fn()) }
  }, [addLog, showToast, refreshBindingCount])

  // 监听测速进度事件
  useEffect(() => {
    const unlisten = listen<TestProgressEvent>('test-progress', (event) => {
      addLog(event.payload.level, event.payload.message)
    })
    return () => { unlisten.then(fn => fn()) }
  }, [addLog])

  const initializeApp = async () => {
    await loadConfig()
    await refreshBindingCount()
    await checkBundledHelper()

    const permissionOk = await checkPermission()

    if (!permissionOk) {
      setShowAdminDialog(true)
      addLog('warning', '没有 hosts 文件写入权限，请选择提升权限或跳过')
      return
    }

    // 恢复已有的测速结果
    try {
      const currentResults = await invoke<EndpointResult[]>('get_current_results')
      if (currentResults && currentResults.length > 0) {
        setResults(currentResults)
        addLog('info', `已加载 ${currentResults.length} 个测速结果`)
      }
    } catch (e) {
      console.error('Failed to get current results:', e)
    }

    // 如果持续优化模式开启且有绑定，自动启动
    try {
      const cfg = await invoke<AppConfig>('get_config')
      if (cfg.continuous_mode) {
        const hasBindings = await invoke<boolean>('has_any_bindings')
        if (hasBindings) {
          await invoke('start_continuous_optimization')
        }
      }
    } catch (e) {
      console.error('Failed to start continuous optimization:', e)
    }
  }

  // ===== 全局测速 =====
  const retestEndpoints = async () => {
    const enabledCount = endpoints.filter((e) => e.enabled).length
    if (enabledCount === 0) {
      addLog('warning', '没有启用的端点，请先添加')
      showToast('warning', '没有启用的端点')
      return
    }

    const permissionOk = await checkPermission()
    if (!permissionOk) {
      if (!userDeclinedAdmin) setShowAdminDialog(true)
      return
    }

    setIsRunning(true)
    setProgress({ current: 0, total: enabledCount, message: '正在测速...' })
    addLog('info', `开始测速，测试 ${enabledCount} 个端点...`)

    try {
      const newResults = await invoke<EndpointResult[]>('start_speed_test', { updateBaseline: true })
      setResults(newResults)
      const successCount = newResults.filter((r) => r.success).length
      setProgress({
        current: newResults.length,
        total: enabledCount,
        message: `测速完成：成功 ${successCount} 个`,
      })
      addLog('success', `测速完成：成功 ${successCount}/${enabledCount} 个`)
      showToast('success', `测速完成: ${successCount} 个可用`)
    } catch (e) {
      console.error('Speed test failed:', e)
      setProgress({ current: 0, total: 0, message: `测速失败: ${e}` })
      addLog('error', `测速失败: ${e}`)
      showToast('error', `测速失败: ${e}`)
    } finally {
      setIsRunning(false)
    }
  }

  // ===== 全局绑定 =====
  const applyAll = async () => {
    const permissionOk = await checkPermission()
    if (!permissionOk) {
      if (!userDeclinedAdmin) setShowAdminDialog(true)
      return
    }

    try {
      const count = await invoke<number>('apply_all_endpoints')
      await refreshBindingCount()
      addLog('success', `已绑定 ${count} 个端点`)
      showToast('success', `已绑定 ${count} 个端点`)
      // 持续优化模式下自动启动（后端已启动，这里确保前端状态同步）
      try {
        const running = await invoke<boolean>('is_continuous_optimization_running')
        setIsOptimizing(running)
      } catch { /* ignore */ }
    } catch (e) {
      console.error('Apply all failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', '全部绑定失败: 需要管理员权限')
      } else {
        addLog('error', `全部绑定失败: ${e}`)
        showToast('error', `绑定失败: ${e}`)
      }
    }
  }

  // ===== 全局解绑 =====
  const unbindAll = async () => {
    const permissionOk = await checkPermission()
    if (!permissionOk) {
      if (!userDeclinedAdmin) setShowAdminDialog(true)
      return
    }

    try {
      const count = await invoke<number>('clear_all_bindings')
      await refreshBindingCount()
      addLog('success', `已解绑 ${count} 个端点`)
      showToast('info', `已解绑 ${count} 个端点`)
      setIsOptimizing(false)
    } catch (e) {
      console.error('Unbind all failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', '全部解绑失败: 需要管理员权限')
      } else {
        addLog('error', `全部解绑失败: ${e}`)
        showToast('error', `解绑失败: ${e}`)
      }
    }
  }

  // ===== 单端点绑定 =====
  const applyEndpoint = async (result: EndpointResult) => {
    try {
      await invoke('apply_endpoint', {
        domain: result.endpoint.domain,
        ip: result.ip,
        latency: result.latency,
      })
      await refreshBindingCount()
      addLog('success', `已绑定: ${result.endpoint.domain} → ${result.ip}`)
      showToast('success', `已绑定 ${result.endpoint.name}`)
    } catch (e) {
      console.error('Apply failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', '绑定失败: 需要管理员权限')
      } else {
        addLog('error', `绑定失败: ${e}`)
        showToast('error', `绑定失败: ${e}`)
      }
    }
  }

  // ===== 单端点解绑 =====
  const unbindEndpoint = async (domain: string) => {
    try {
      await invoke('unbind_endpoint', { domain })
      await refreshBindingCount()
      addLog('success', `已解绑: ${domain}`)
      showToast('info', `已解绑 ${domain}`)
    } catch (e) {
      console.error('Unbind failed:', e)
      if (handlePermissionError(e)) {
        addLog('error', '解绑失败: 需要管理员权限')
      } else {
        addLog('error', `解绑失败: ${e}`)
        showToast('error', `解绑失败: ${e}`)
      }
    }
  }

  // ===== 单端点测速 =====
  const testSingleEndpoint = async (endpoint: Endpoint) => {
    const domain = endpoint.domain

    setTestingDomains(prev => new Set(prev).add(domain))
    addLog('info', `正在单独测速: ${endpoint.name} (${domain})`)

    try {
      const result = await invoke<EndpointResult>('test_single_endpoint', { endpoint })

      setResults(prev => {
        const existing = prev.findIndex(r => r.endpoint.domain === domain)
        if (existing >= 0) {
          const updated = [...prev]
          updated[existing] = result
          return updated
        }
        return [...prev, result]
      })

      if (result.success) {
        addLog('success', `测速完成: ${endpoint.name} → ${result.ip} (${result.latency.toFixed(0)}ms)`)
        showToast('success', `${endpoint.name}: ${result.latency.toFixed(0)}ms`)
      } else {
        addLog('warning', `测速失败: ${endpoint.name} - ${result.error || '未知错误'}`)
        showToast('warning', `${endpoint.name} 测速失败`)
      }
    } catch (e) {
      console.error('Single endpoint test failed:', e)
      addLog('error', `测速出错: ${endpoint.name} - ${e}`)
      showToast('error', `${endpoint.name} 测速出错`)
    } finally {
      setTestingDomains(prev => {
        const next = new Set(prev)
        next.delete(domain)
        return next
      })
    }
  }

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
      <ToastContainer toasts={toasts} onClose={removeToast} />

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
                <p className="text-sm text-gray-500">
                  {isMacOS
                    ? (hasBundledHelper ? '需要安装 Helper 组件' : '需要手动安装 Helper')
                    : '无法启用加速功能，需要管理员授权'
                  }
                </p>
              </div>
            </div>

            {isMacOS ? (
              <div className="mb-4">
                {hasBundledHelper ? (
                  <p className="text-sm text-gray-600">
                    点击下方按钮安装 Helper，系统会弹出密码输入框。安装后即可无感修改 hosts 文件。
                  </p>
                ) : (
                  <p className="text-sm text-gray-600">
                    未找到内置 Helper，请从
                    <a
                      href="https://github.com/wangwingzero/anyFAST/releases"
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-500 hover:underline mx-1"
                    >
                      GitHub Release
                    </a>
                    下载并手动安装。
                  </p>
                )}
              </div>
            ) : (
              <div className="mb-4">
                {isRunningAsAdmin ? (
                  <>
                    <p className="text-sm text-gray-600 mb-3">
                      当前已以管理员身份运行，但 Service 未安装或未启动。点击下方按钮安装并启动 Service：
                    </p>
                    <p className="text-xs text-gray-500 mt-2">
                      安装后 Service 将以系统权限运行，无需每次以管理员启动。
                    </p>
                  </>
                ) : (
                  <>
                    <p className="text-sm text-gray-600 mb-3">
                      修改 hosts 文件需要管理员授权。建议按下面顺序尝试：
                    </p>
                    <ul className="text-sm text-gray-600 space-y-2 ml-4">
                      <li className="flex items-start gap-2">
                        <span className="text-gray-400 mt-0.5">•</span>
                        <span>先点"仅重试连接"（适用于刚启动/刚安装）</span>
                      </li>
                      <li className="flex items-start gap-2">
                        <span className="text-gray-400 mt-0.5">•</span>
                        <span>仍失败，再点"一键授权并重启"（会安装 Service 并重启）</span>
                      </li>
                    </ul>
                    <p className="text-xs text-gray-500 mt-3">
                      仍无法连接可能是安全软件拦截，请尝试重新安装最新版或手动安装 Service。
                    </p>
                  </>
                )}
              </div>
            )}

            {isMacOS ? (
              <div className="flex gap-3">
                <button
                  onClick={declineAdmin}
                  className="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
                >
                  跳过（仅查看）
                </button>
                {hasBundledHelper ? (
                  <button
                    onClick={installMacOSHelper}
                    disabled={isInstallingHelper}
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-orange-500 rounded-xl hover:bg-orange-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                  >
                    {isInstallingHelper ? (
                      <>
                        <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
                          <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                          <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                        </svg>
                        安装中...
                      </>
                    ) : (
                      '安装 Helper'
                    )}
                  </button>
                ) : (
                  <a
                    href="https://github.com/wangwingzero/anyFAST/releases"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-xl hover:bg-blue-600 transition-colors text-center"
                  >
                    前往下载
                  </a>
                )}
              </div>
            ) : (
              <div className="flex flex-col gap-3">
                {isRunningAsAdmin ? (
                  <>
                    <div className="flex flex-col gap-1">
                      <button
                        onClick={installWindowsService}
                        disabled={isInstallingService}
                        className="w-full px-4 py-2.5 text-sm font-medium text-white bg-apple-blue rounded-xl hover:bg-blue-600 transition-colors flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        {isInstallingService ? (
                          <>
                            <svg className="animate-spin h-4 w-4" viewBox="0 0 24 24">
                              <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" fill="none" />
                              <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
                            </svg>
                            安装中...
                          </>
                        ) : (
                          <>
                            安装 Service
                            <span className="text-[10px] px-1.5 py-0.5 rounded bg-white/20">推荐</span>
                          </>
                        )}
                      </button>
                      <p className="text-[11px] text-gray-500 text-center">安装后无需每次以管理员启动</p>
                    </div>
                    <div className="flex flex-col gap-1">
                      <button
                        onClick={async () => {
                          await invoke('refresh_service_status')
                          const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
                          if (status.hasPermission) {
                            setShowAdminDialog(false)
                            setHasPermission(true)
                            addLog('success', '已连接到 anyFAST Service')
                            setSidebarRefreshTrigger(prev => prev + 1)
                          } else {
                            addLog('warning', 'Service 仍未连接，请尝试安装 Service')
                          }
                        }}
                        className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-xl hover:bg-blue-600 transition-colors"
                      >
                        仅重试连接
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="flex flex-col gap-1">
                      <button
                        onClick={restartAsAdmin}
                        className="w-full px-4 py-2 text-sm font-medium text-white bg-orange-500 rounded-xl hover:bg-orange-600 transition-colors flex items-center justify-center gap-2"
                      >
                        一键授权并重启
                        <span className="text-[10px] px-1.5 py-0.5 rounded bg-white/20">推荐</span>
                      </button>
                      <p className="text-[11px] text-gray-500 text-center">会弹出系统授权窗口并重启应用</p>
                    </div>
                    <div className="flex flex-col gap-1">
                      <button
                        onClick={async () => {
                          await invoke('refresh_service_status')
                          const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
                          if (status.hasPermission) {
                            setShowAdminDialog(false)
                            setHasPermission(true)
                            addLog('success', '已连接到 anyFAST Service')
                            setSidebarRefreshTrigger(prev => prev + 1)
                          } else {
                            addLog('warning', 'Service 仍未连接，请稍后重试')
                          }
                        }}
                        className="w-full px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-xl hover:bg-blue-600 transition-colors"
                      >
                        仅重试连接
                      </button>
                      <p className="text-[11px] text-gray-500 text-center">适用于刚启动/刚安装</p>
                    </div>
                  </>
                )}
                <button
                  onClick={declineAdmin}
                  className="w-full px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
                >
                  暂不授权（仅查看）
                </button>
              </div>
            )}
          </div>
        </div>
      )}

      <Sidebar currentView={currentView} onNavigate={setCurrentView} refreshTrigger={sidebarRefreshTrigger} />

      <main className="flex-1 overflow-auto min-w-0">
        {currentView === 'dashboard' && (
          <Dashboard
            endpoints={endpoints}
            results={results}
            isRunning={isRunning}
            bindingCount={bindingCount}
            testingDomains={testingDomains}
            config={config}
            isOptimizing={isOptimizing}
            onApply={applyEndpoint}
            onApplyAll={applyAll}
            onUnbindAll={unbindAll}
            onUnbindEndpoint={unbindEndpoint}
            onRetest={retestEndpoints}
            onTestSingle={testSingleEndpoint}
            onEndpointsChange={setEndpoints}
            onSaveConfig={saveConfigWithEndpoints}
            onConfigChange={setConfig}
          />
        )}
        {currentView === 'history' && <HistoryView />}
        {currentView === 'logs' && (
          <Logs logs={logs} onClear={clearLogs} />
        )}
        {currentView === 'settings' && (
          <Settings
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
