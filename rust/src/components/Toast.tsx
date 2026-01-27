import { useEffect, useState } from 'react'
import { CheckCircle2, XCircle, Info, AlertTriangle, X } from 'lucide-react'

export type ToastType = 'success' | 'error' | 'info' | 'warning'

export interface ToastData {
  id: number
  type: ToastType
  message: string
}

interface ToastProps {
  toast: ToastData
  onClose: (id: number) => void
}

function Toast({ toast, onClose }: ToastProps) {
  const [isExiting, setIsExiting] = useState(false)

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsExiting(true)
      setTimeout(() => onClose(toast.id), 300)
    }, 3000)
    return () => clearTimeout(timer)
  }, [toast.id, onClose])

  const iconMap = {
    success: <CheckCircle2 className="w-5 h-5 text-apple-green" />,
    error: <XCircle className="w-5 h-5 text-apple-red" />,
    info: <Info className="w-5 h-5 text-apple-blue" />,
    warning: <AlertTriangle className="w-5 h-5 text-apple-orange" />,
  }

  const bgMap = {
    success: 'bg-apple-green/10 border-apple-green/20',
    error: 'bg-apple-red/10 border-apple-red/20',
    info: 'bg-apple-blue/10 border-apple-blue/20',
    warning: 'bg-apple-orange/10 border-apple-orange/20',
  }

  return (
    <div
      className={`
        flex items-center gap-3 px-4 py-3 rounded-apple-lg border backdrop-blur-xl shadow-lg
        ${bgMap[toast.type]}
        ${isExiting ? 'animate-toast-out' : 'animate-toast-in'}
      `}
    >
      {iconMap[toast.type]}
      <span className="text-sm font-medium text-apple-gray-600 flex-1">{toast.message}</span>
      <button
        onClick={() => {
          setIsExiting(true)
          setTimeout(() => onClose(toast.id), 300)
        }}
        className="p-1 rounded-md hover:bg-apple-gray-200/50 transition-colors"
      >
        <X className="w-4 h-4 text-apple-gray-400" />
      </button>
    </div>
  )
}

interface ToastContainerProps {
  toasts: ToastData[]
  onClose: (id: number) => void
}

export function ToastContainer({ toasts, onClose }: ToastContainerProps) {
  if (toasts.length === 0) return null

  return (
    <div className="fixed top-4 right-4 z-50 flex flex-col gap-2 max-w-sm">
      {toasts.map((toast) => (
        <Toast key={toast.id} toast={toast} onClose={onClose} />
      ))}
    </div>
  )
}
