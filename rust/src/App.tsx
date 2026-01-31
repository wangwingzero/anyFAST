import { useState, useEffect, useCallback } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
// import { relaunch } from '@tauri-apps/plugin-process' // 保留以备将来使用
import { Sidebar } from './components/Sidebar'
import { Dashboard } from './components/Dashboard'
import { Settings } from './components/Settings'
import { Logs } from './components/Logs'
import { HistoryView } from './components/HistoryView'
import { ToastContainer, ToastData, ToastType } from './components'
import { Endpoint, EndpointResult, AppConfig, LogEntry, EndpointHealth, WorkflowResult } from './types'

type View = 'dashboard' | 'settings' | 'logs' | 'history'

let toastIdCounter = 0

// 检测操作系统
const isMacOS = navigator.userAgent.includes('Mac')
// const isWindows = navigator.userAgent.includes('Windows') // 保留以备将来使用

// 检查是否是权限错误
const isPermissionError = (error: unknown): boolean => {
  const errorStr = String(error).toLowerCase()
  return errorStr.includes('permission denied') || 
         errorStr.includes('access denied') ||
         errorStr.includes('administrator') ||
         errorStr.includes('拒绝访问') ||
         errorStr.includes('os error 5')  // Windows ERROR_ACCESS_DENIED
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
  const [isWorking, setIsWorking] = useState(false)
  // 用户是否已拒绝管理员权限提升（当前会话内记住）
  const [userDeclinedAdmin, setUserDeclinedAdmin] = useState(false)
  // 权限状态
  const [, setHasPermission] = useState<boolean | null>(null)
  // 是否正在安装 helper
  const [isInstallingHelper, setIsInstallingHelper] = useState(false)
  // 是否有内置 helper（macOS）
  const [hasBundledHelper, setHasBundledHelper] = useState(false)

  const showToast = useCallback((type: ToastType, message: string) => {
    const id = ++toastIdCounter
    setToasts((prev) => [...prev, { id, type, message }])
  }, [])

  const removeToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id))
  }, [])

  // 添加日志（放在前面，因为其他函数依赖它）
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

  // 处理权限错误，提示用户重启为管理员
  // 如果用户已经拒绝过，不再弹出对话框
  const handlePermissionError = useCallback((error: unknown) => {
    if (isPermissionError(error)) {
      if (!userDeclinedAdmin) {
        setShowAdminDialog(true)
      }
      return true
    }
    return false
  }, [userDeclinedAdmin])

  // 以管理员身份重启
  const restartAsAdmin = useCallback(async () => {
    try {
      await invoke('restart_as_admin')
    } catch {
      // 用户取消或出错
      setShowAdminDialog(false)
    }
  }, [])

  // macOS: 安装 helper（使用 osascript 弹出系统密码框）
  // 注意：这个函数内部调用了 checkPermission 和 checkWorkflowStatus，
  // 由于 JavaScript 函数提升，这些函数在运行时是可用的
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
        
        // 安装成功后，检查权限并继续工作流
        // 不需要重启，因为后端已经刷新了缓存
        setTimeout(async () => {
          try {
            const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
            if (status.hasPermission) {
              addLog('info', '权限验证成功，正在启动工作流...')
              // 直接调用自动启动工作流
              autoStartWorkflow()
            }
          } catch (e) {
            console.error('Failed to verify permission after helper install:', e)
          }
        }, 500)
      } else {
        // Helper 不在 bundle 中，提示手动安装
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

  // 用户拒绝管理员权限
  const declineAdmin = useCallback(() => {
    setUserDeclinedAdmin(true)
    setShowAdminDialog(false)
    addLog('info', '已跳过管理员权限，部分功能可能受限')
  }, [addLog])

  // 检查权限状态
  const checkPermission = useCallback(async (): Promise<boolean> => {
    try {
      const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
      setHasPermission(status.hasPermission)
      if (status.isUsingService) {
        addLog('info', isMacOS ? '已连接到 anyFAST Helper' : '已连接到 anyFAST Service')
      } else if (status.hasPermission) {
        addLog('info', '以管理员身份运行')
      }
      return status.hasPermission
    } catch (e) {
      console.error('Failed to check permission:', e)
      setHasPermission(false)
      return false
    }
  }, [addLog])

  // 检查是否有内置 helper（macOS）
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

  // 清空日志
  const clearLogs = useCallback(() => {
    setLogs([])
    addLog('info', '日志已清空')
  }, [addLog])

  useEffect(() => {
    initializeApp()

    // 监听健康检查结果事件
    const unlistenHealth = listen<{ endpoints_health: EndpointHealth[] }>('health-check-result', (event) => {
      setHealthStatus(event.payload.endpoints_health)
    })

    return () => {
      unlistenHealth.then(fn => fn())
    }
  }, [])

  // 应用初始化流程
  const initializeApp = async () => {
    await loadConfig()
    await refreshBindingCount()
    await checkBundledHelper()
    
    // 先检查权限状态
    const permissionOk = await checkPermission()
    
    if (!permissionOk) {
      // 没有权限，显示对话框让用户选择
      setShowAdminDialog(true)
      addLog('warning', '没有 hosts 文件写入权限，请选择提升权限或跳过')
      return
    }
    
    // 有权限，继续检查工作流状态
    await checkWorkflowStatus()
  }

  // 检查工作流运行状态，如果未运行则自动启动
  const checkWorkflowStatus = async () => {
    try {
      const running = await invoke<boolean>('is_workflow_running')
      setIsWorking(running)
      if (running) {
        addLog('info', '检测到工作流正在运行')
        // 获取当前的测速结果
        try {
          const currentResults = await invoke<EndpointResult[]>('get_current_results')
          if (currentResults && currentResults.length > 0) {
            setResults(currentResults)
            addLog('info', `已加载 ${currentResults.length} 个测速结果`)
          }
        } catch (e) {
          console.error('Failed to get current results:', e)
        }
      } else {
        // 工作流未运行，自动启动
        autoStartWorkflow()
      }
    } catch (e) {
      console.error('Failed to check workflow status:', e)
      // 检查失败也尝试自动启动
      autoStartWorkflow()
    }
  }

  // 自动启动工作流
  const autoStartWorkflow = async () => {
    // 等待配置加载完成
    await new Promise(resolve => setTimeout(resolve, 500))
    
    try {
      const config = await invoke<AppConfig>('get_config')
      const enabledCount = config.endpoints.filter((e: Endpoint) => e.enabled).length
      
      if (enabledCount === 0) {
        addLog('info', '没有启用的端点，跳过自动启动')
        return
      }

      setIsRunning(true)
      setProgress({ current: 0, total: enabledCount, message: '正在自动启动工作流...' })
      addLog('info', `自动启动工作流，测试 ${enabledCount} 个端点...`)
      
      const result = await invoke<WorkflowResult>('start_workflow')
      setIsWorking(true)
      setResults(result.results)
      await refreshBindingCount()
      
      setProgress({ 
        current: result.testCount, 
        total: result.testCount, 
        message: `已应用 ${result.appliedCount} 个绑定` 
      })
      addLog('success', `工作流已自动启动: 测试 ${result.testCount} 个端点，成功 ${result.successCount} 个，应用 ${result.appliedCount} 个绑定`)
    } catch (e) {
      console.error('Auto start workflow failed:', e)
      addLog('warning', `自动启动失败: ${e}`)
    } finally {
      setIsRunning(false)
    }
  }

  // 切换工作流状态（启动/停止）
  const toggleWorkflow = async () => {
    if (isWorking) {
      // 停止工作流
      setIsRunning(true)
      try {
        addLog('info', '正在停止工作流...')
        const clearedCount = await invoke<number>('stop_workflow')
        setIsWorking(false)
        await refreshBindingCount()
        setProgress({ current: 0, total: 0, message: '已停止' })
        addLog('success', `工作流已停止，清除了 ${clearedCount} 个绑定`)
        showToast('info', `已停止，清除了 ${clearedCount} 个绑定`)
        setHealthStatus([])
      } catch (e) {
        console.error('Stop workflow failed:', e)
        if (handlePermissionError(e)) {
          addLog('error', '停止工作流失败: 需要管理员权限')
        } else {
          addLog('error', `停止工作流失败: ${e}`)
          showToast('error', `停止失败: ${e}`)
        }
      } finally {
        setIsRunning(false)
      }
    } else {
      // 启动工作流
      const enabledCount = endpoints.filter((e) => e.enabled).length
      if (enabledCount === 0) {
        addLog('warning', '没有启用的端点，请先添加')
        showToast('warning', '没有启用的端点')
        return
      }

      setIsRunning(true)
      try {
        setProgress({ current: 0, total: enabledCount, message: '正在启动工作流...' })
        addLog('info', `正在启动工作流，测试 ${enabledCount} 个端点...`)
        
        const result = await invoke<WorkflowResult>('start_workflow')
        setIsWorking(true)
        setResults(result.results)
        await refreshBindingCount()
        
        setProgress({ 
          current: result.testCount, 
          total: result.testCount, 
          message: `已应用 ${result.appliedCount} 个绑定` 
        })
        addLog('success', `工作流已启动: 测试 ${result.testCount} 个端点，成功 ${result.successCount} 个，应用 ${result.appliedCount} 个绑定`)
        showToast('success', `已启动，应用了 ${result.appliedCount} 个绑定`)
      } catch (e) {
        console.error('Start workflow failed:', e)
        if (handlePermissionError(e)) {
          addLog('error', '启动工作流失败: 需要管理员权限')
        } else {
          setProgress({ current: 0, total: 0, message: `启动失败: ${e}` })
          addLog('error', `启动工作流失败: ${e}`)
          showToast('error', `启动失败: ${e}`)
        }
      } finally {
        setIsRunning(false)
      }
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

  const refreshBindingCount = async () => {
    try {
      const count = await invoke<number>('get_binding_count')
      setBindingCount(count)
    } catch (e) {
      console.error('Failed to get binding count:', e)
    }
  }

  // [已移除] startTest - 由 toggleWorkflow 中的 start_workflow 替代
  // 原有的手动测速功能已整合到简化工作流中

  // [已移除] stopTest - 由 toggleWorkflow 中的 stop_workflow 替代
  // 原有的手动停止功能已整合到简化工作流中

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

  // [已移除] applyAll - 由 toggleWorkflow 中的 start_workflow 替代
  // const applyAll = async () => {
  //   try {
  //     const count = await invoke<number>('apply_all_endpoints')
  //     await refreshBindingCount()
  //     setProgress({ ...progress, message: `已绑定 ${count} 个端点` })
  //     addLog('success', `一键应用完成: 已绑定 ${count} 个端点`)
  //     showToast('success', `已成功绑定 ${count} 个端点`)
  //   } catch (e) {
  //     console.error('Apply all failed:', e)
  //     if (handlePermissionError(e)) {
  //       addLog('error', `一键应用失败: 需要管理员权限`)
  //     } else {
  //       setProgress({ ...progress, message: `绑定失败: ${e}` })
  //       addLog('error', `一键应用失败: ${e}`)
  //       showToast('error', `一键应用失败: ${e}`)
  //     }
  //   }
  // }

  // [已移除] clearBindings - 由 toggleWorkflow 中的 stop_workflow 替代
  // const clearBindings = async () => {
  //   try {
  //     const count = await invoke<number>('clear_all_bindings')
  //     await refreshBindingCount()
  //     setProgress({ ...progress, message: `已清除 ${count} 个绑定` })
  //     addLog('info', `已清除 ${count} 个绑定`)
  //     showToast('info', `已清除 ${count} 个绑定`)
  //   } catch (e) {
  //     console.error('Clear failed:', e)
  //     if (handlePermissionError(e)) {
  //       addLog('error', `清除绑定失败: 需要管理员权限`)
  //     } else {
  //       setProgress({ ...progress, message: `清除失败: ${e}` })
  //       addLog('error', `清除绑定失败: ${e}`)
  //       showToast('error', `清除失败: ${e}`)
  //     }
  //   }
  // }

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
                <p className="text-sm text-gray-500">
                  {isMacOS 
                    ? (hasBundledHelper ? '需要安装 Helper 组件' : '需要手动安装 Helper')
                    : 'Service 连接失败，需要提升权限'
                  }
                </p>
              </div>
            </div>
            
            {isMacOS ? (
              // macOS 说明
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
              // Windows 说明
              <div className="mb-4">
                <p className="text-sm text-gray-600 mb-3">
                  修改 hosts 文件需要管理员权限。Service 未能连接，可能原因：
                </p>
                <ul className="text-sm text-gray-600 space-y-2 ml-4">
                  <li className="flex items-start gap-2">
                    <span className="text-gray-400 mt-0.5">•</span>
                    <span>Service 尚未启动（请稍等几秒后重试）</span>
                  </li>
                  <li className="flex items-start gap-2">
                    <span className="text-gray-400 mt-0.5">•</span>
                    <span>旧版本升级（请重新安装最新版）</span>
                  </li>
                </ul>
              </div>
            )}
            
            <div className="flex gap-3">
              <button
                onClick={declineAdmin}
                className="flex-1 px-4 py-2 text-sm font-medium text-gray-700 bg-gray-100 rounded-xl hover:bg-gray-200 transition-colors"
              >
                跳过（仅查看）
              </button>
              {isMacOS ? (
                hasBundledHelper ? (
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
                )
              ) : (
                <div className="flex gap-2 flex-1">
                  <button
                    onClick={async () => {
                      await invoke('refresh_service_status')
                      const status = await invoke<{ hasPermission: boolean; isUsingService: boolean }>('get_permission_status')
                      if (status.hasPermission) {
                        setShowAdminDialog(false)
                        setHasPermission(true)
                        addLog('success', '已连接到 anyFAST Service')
                      } else {
                        addLog('warning', 'Service 仍未连接，请稍后重试')
                      }
                    }}
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-blue-500 rounded-xl hover:bg-blue-600 transition-colors"
                  >
                    重试连接
                  </button>
                  <button
                    onClick={restartAsAdmin}
                    className="flex-1 px-4 py-2 text-sm font-medium text-white bg-orange-500 rounded-xl hover:bg-orange-600 transition-colors"
                  >
                    管理员重启
                  </button>
                </div>
              )}
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
            isWorking={isWorking}
            progress={progress}
            bindingCount={bindingCount}
            healthStatus={healthStatus}
            onApply={applyEndpoint}
            onToggleWorkflow={toggleWorkflow}
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
