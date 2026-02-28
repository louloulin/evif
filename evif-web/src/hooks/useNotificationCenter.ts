import { useState, useCallback } from 'react'
import { toast } from '@/hooks/use-toast'
import type { Notification } from '@/components/NotificationCenter'

let notificationId = 0

export const useNotificationCenter = () => {
  const [notifications, setNotifications] = useState<Notification[]>([])
  const [open, setOpen] = useState(false)

  const addNotification = useCallback((
    type: Notification['type'],
    title: string,
    message?: string
  ) => {
    const id = `notification-${++notificationId}`

    const notification: Notification = {
      id,
      type,
      title,
      message,
      timestamp: new Date(),
      read: false,
    }

    setNotifications(prev => [notification, ...prev])

    // 同时显示toast
    toast({
      title,
      description: message,
      variant: type === 'error' ? 'destructive' : 'default',
    })

    return id
  }, [])

  const markAsRead = useCallback((id: string) => {
    setNotifications(prev =>
      prev.map(n =>
        n.id === id ? { ...n, read: true } : n
      )
    )
  }, [])

  const markAllAsRead = useCallback(() => {
    setNotifications(prev =>
      prev.map(n => ({ ...n, read: true }))
    )
  }, [])

  const clearNotification = useCallback((id: string) => {
    setNotifications(prev => prev.filter(n => n.id !== id))
  }, [])

  const clearAll = useCallback(() => {
    setNotifications([])
  }, [])

  const success = useCallback((title: string, message?: string) => {
    return addNotification('success', title, message)
  }, [addNotification])

  const error = useCallback((title: string, message?: string) => {
    return addNotification('error', title, message)
  }, [addNotification])

  const warning = useCallback((title: string, message?: string) => {
    return addNotification('warning', title, message)
  }, [addNotification])

  const info = useCallback((title: string, message?: string) => {
    return addNotification('info', title, message)
  }, [addNotification])

  const unreadCount = notifications.filter(n => !n.read).length

  return {
    notifications,
    open,
    setOpen,
    addNotification,
    markAsRead,
    markAllAsRead,
    clearNotification,
    clearAll,
    success,
    error,
    warning,
    info,
    unreadCount,
  }
}
