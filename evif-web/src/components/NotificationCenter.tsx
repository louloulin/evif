import React, { useState, useMemo } from 'react'
import { Bell, X, CheckCircle, AlertCircle, Info, XCircle, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ScrollArea } from '@/components/ui/scroll-area'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'

export type ToastType = 'success' | 'error' | 'warning' | 'info'

export interface Notification {
  id: string
  type: ToastType
  title: string
  message?: string
  timestamp: Date
  read: boolean
}

interface NotificationCenterProps {
  open: boolean
  onClose: () => void
  notifications: Notification[]
  onMarkRead: (id: string) => void
  onMarkAllRead: () => void
  onClear: (id: string) => void
  onClearAll: () => void
}

export const NotificationCenter: React.FC<NotificationCenterProps> = ({
  open,
  onClose,
  notifications,
  onMarkRead,
  onMarkAllRead,
  onClear,
  onClearAll,
}) => {
  const [filter, setFilter] = useState<ToastType | 'all'>('all')

  const filteredNotifications = useMemo(() => {
    if (filter === 'all') return notifications
    return notifications.filter(n => n.type === filter)
  }, [notifications, filter])

  const unreadCount = notifications.filter(n => !n.read).length

  const getIcon = (type: ToastType) => {
    switch (type) {
      case 'success':
        return <CheckCircle className="h-5 w-5 text-green-500" />
      case 'error':
        return <XCircle className="h-5 w-5 text-red-500" />
      case 'warning':
        return <AlertCircle className="h-5 w-5 text-yellow-500" />
      case 'info':
        return <Info className="h-5 w-5 text-blue-500" />
    }
  }

  const getTypeLabel = (type: ToastType) => {
    switch (type) {
      case 'success':
        return '成功'
      case 'error':
        return '错误'
      case 'warning':
        return '警告'
      case 'info':
        return '信息'
    }
  }

  const formatTime = (date: Date) => {
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const seconds = Math.floor(diff / 1000)
    const minutes = Math.floor(seconds / 60)
    const hours = Math.floor(minutes / 60)
    const days = Math.floor(hours / 24)

    if (seconds < 60) return '刚刚'
    if (minutes < 60) return `${minutes}分钟前`
    if (hours < 24) return `${hours}小时前`
    if (days < 7) return `${days}天前`
    return date.toLocaleDateString('zh-CN')
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-md">
        <DialogHeader>
          <DialogTitle className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Bell className="h-5 w-5" />
              <span>通知中心</span>
              {unreadCount > 0 && (
                <span className="ml-2 px-2 py-0.5 text-xs bg-primary text-primary-foreground rounded-full">
                  {unreadCount}
                </span>
              )}
            </div>
          </DialogTitle>
          <DialogDescription>
            查看和管理系统通知
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          {/* 过滤器 */}
          <div className="flex gap-2 flex-wrap">
            <Button
              variant={filter === 'all' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter('all')}
            >
              全部 ({notifications.length})
            </Button>
            <Button
              variant={filter === 'success' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter('success')}
            >
              成功
            </Button>
            <Button
              variant={filter === 'error' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter('error')}
            >
              错误
            </Button>
            <Button
              variant={filter === 'warning' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter('warning')}
            >
              警告
            </Button>
            <Button
              variant={filter === 'info' ? 'default' : 'outline'}
              size="sm"
              onClick={() => setFilter('info')}
            >
              信息
            </Button>
          </div>

          {/* 操作按钮 */}
          <div className="flex gap-2">
            {unreadCount > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={onMarkAllRead}
                className="flex-1"
              >
                全部已读
              </Button>
            )}
            {notifications.length > 0 && (
              <Button
                variant="outline"
                size="sm"
                onClick={onClearAll}
                className="flex-1"
              >
                清空全部
              </Button>
            )}
          </div>

          {/* 通知列表 */}
          <ScrollArea className="h-[400px] border rounded-md">
            {filteredNotifications.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-32 text-muted-foreground">
                <Bell className="h-8 w-8 mb-2 opacity-50" />
                <p>暂无通知</p>
              </div>
            ) : (
              <div className="p-2 space-y-2">
                {filteredNotifications.map((notification) => (
                  <div
                    key={notification.id}
                    className={`
                      p-3 rounded-lg border transition-colors
                      ${notification.read ? 'bg-muted/30 opacity-70' : 'bg-background'}
                    `}
                  >
                    <div className="flex items-start gap-4">
                      {/* 图标 */}
                      <div className="shrink-0 mt-0.5">
                        {getIcon(notification.type)}
                      </div>

                      {/* 内容 */}
                      <div className="flex-1 min-w-0">
                        <div className="flex items-center justify-between gap-2 mb-1">
                          <h4 className="font-medium text-sm truncate">
                            {notification.title}
                          </h4>
                          <span className="text-xs text-muted-foreground shrink-0">
                            {formatTime(notification.timestamp)}
                          </span>
                        </div>
                        {notification.message && (
                          <p className="text-sm text-muted-foreground line-clamp-2">
                            {notification.message}
                          </p>
                        )}
                        <div className="flex items-center gap-2 mt-2">
                          <span className="text-xs px-2 py-0.5 rounded bg-muted">
                            {getTypeLabel(notification.type)}
                          </span>
                          {!notification.read && (
                            <span className="text-xs text-primary">未读</span>
                          )}
                        </div>
                      </div>

                      {/* 操作按钮 */}
                      <div className="flex flex-col gap-1 shrink-0">
                        {!notification.read && (
                          <Button
                            variant="ghost"
                            size="sm"
                            className="h-7 w-7 p-0"
                            onClick={() => onMarkRead(notification.id)}
                            title="标记为已读"
                          >
                            <CheckCircle className="h-3.5 w-3.5" />
                          </Button>
                        )}
                        <Button
                          variant="ghost"
                          size="sm"
                          className="h-7 w-7 p-0"
                          onClick={() => onClear(notification.id)}
                          title="删除"
                        >
                          <X className="h-3.5 w-3.5" />
                        </Button>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            )}
          </ScrollArea>
        </div>
      </DialogContent>
    </Dialog>
  )
}
