import React from 'react'
import { Activity as ActivityIcon, Clock, FileText, GitCommit, Share2, MessageSquare } from 'lucide-react'
import type { Activity } from '@/types/collaboration'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Badge } from '@/components/ui/badge'

interface ActivityFeedProps {
  activities: Activity[]
  filePath?: string
  showFilters?: boolean
}

export const ActivityFeed: React.FC<ActivityFeedProps> = ({
  activities,
  filePath,
  showFilters = true,
}) => {
  const [filter, setFilter] = React.useState<'all' | 'file' | 'share' | 'comment'>('all')
  const [selectedType, setSelectedType] = React.useState<string | null>(null)

  // 过滤活动
  const filteredActivities = React.useMemo(() => {
    let result = activities

    // 按文件路径过滤
    if (filePath) {
      result = result.filter(a => a.filePath === filePath)
    }

    // 按类型过滤
    if (filter !== 'all') {
      result = result.filter(a => {
        switch (filter) {
          case 'file':
            return ['create', 'update', 'delete'].includes(a.type)
          case 'share':
            return a.type === 'share'
          case 'comment':
            return a.type === 'comment'
          default:
            return true
        }
      })
    }

    // 按时间排序（最新的在前）
    return result.sort((a, b) => {
      const timeA = new Date(a.timestamp).getTime()
      const timeB = new Date(b.timestamp).getTime()
      return timeB - timeA
    })
  }, [activities, filePath, filter])

  const getActivityIcon = (activity: Activity) => {
    const icons: Record<string, React.ReactNode> = {
      create: <FileText className="h-4 w-4 text-blue-500" />,
      update: <GitCommit className="h-4 w-4 text-green-500" />,
      delete: <FileText className="h-4 w-4 text-red-500" />,
      share: <Share2 className="h-4 w-4 text-purple-500" />,
      comment: <MessageSquare className="h-4 w-4 text-yellow-500" />,
    }
    return icons[activity.type] || <ActivityIcon className="h-4 w-4" />
  }

  const getActivityColor = (activity: Activity): string => {
    const colors: Record<string, string> = {
      create: 'bg-blue-500',
      update: 'bg-green-500',
      delete: 'bg-red-500',
      share: 'bg-purple-500',
      comment: 'bg-yellow-500',
    }
    return colors[activity.type] || 'bg-gray-500'
  }

  const getActivityBadge = (activity: Activity): string => {
    const badges: Record<string, string> = {
      create: '创建',
      update: '更新',
      delete: '删除',
      share: '分享',
      comment: '评论',
    }
    return badges[activity.type] || '操作'
  }

  const formatDate = (date: Date): string => {
    const now = new Date()
    const diff = now.getTime() - date.getTime()
    const minutes = Math.floor(diff / (1000 * 60))
    const hours = Math.floor(diff / (1000 * 60 * 60))
    const days = Math.floor(diff / (1000 * 60 * 60 * 24))

    if (days > 0) return `${days} 天前`
    if (hours > 0) return `${hours} 小时前`
    if (minutes > 0) return `${minutes} 分钟前`
    return '刚刚'
  }

  const formatTime = (date: Date): string => {
    return date.toLocaleTimeString('zh-CN', {
      hour: '2-digit',
      minute: '2-digit',
    })
  }

  // 按日期分组
  const groupedActivities = React.useMemo(() => {
    const groups: Record<string, Activity[]> = {}

    filteredActivities.forEach((activity) => {
      const date = new Date(activity.timestamp)
      const dateKey = date.toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      })

      if (!groups[dateKey]) {
        groups[dateKey] = []
      }
      groups[dateKey].push(activity)
    })

    return groups
  }, [filteredActivities])

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <ActivityIcon className="h-5 w-5" />
            活动历史
          </CardTitle>
          {showFilters && (
            <div className="flex gap-2">
              <Badge
                variant={filter === 'all' ? 'default' : 'outline'}
                className="cursor-pointer"
                onClick={() => setFilter('all')}
              >
                全部
              </Badge>
              <Badge
                variant={filter === 'file' ? 'default' : 'outline'}
                className="cursor-pointer"
                onClick={() => setFilter('file')}
              >
                文件
              </Badge>
              <Badge
                variant={filter === 'share' ? 'default' : 'outline'}
                className="cursor-pointer"
                onClick={() => setFilter('share')}
              >
                分享
              </Badge>
              <Badge
                variant={filter === 'comment' ? 'default' : 'outline'}
                className="cursor-pointer"
                onClick={() => setFilter('comment')}
              >
                评论
              </Badge>
            </div>
          )}
        </div>
        {filePath && (
          <div className="text-sm text-muted-foreground mt-2">
            当前文件: {filePath}
          </div>
        )}
      </CardHeader>

      <CardContent>
        <ScrollArea className="h-[500px]">
          {Object.keys(groupedActivities).length === 0 ? (
            <div className="flex items-center justify-center h-32 text-muted-foreground">
              暂无活动记录
            </div>
          ) : (
            <div className="space-y-6 pr-4">
              {Object.entries(groupedActivities).map(([date, items]) => (
                <div key={date}>
                  {/* 日期标题 */}
                  <div className="text-sm font-semibold text-muted-foreground mb-3 sticky top-0 bg-background py-2">
                    {date}
                  </div>

                  {/* 活动列表 */}
                  <div className="space-y-2 pl-4 border-l-2 border-muted">
                    {items.map((activity, index) => (
                      <div
                        key={activity.id}
                        className="flex items-start gap-4 py-2 hover:bg-accent/50 transition-colors cursor-pointer rounded-r"
                        onClick={() => {
                          // 点击活动可以查看详情
                          setSelectedType(activity.type)
                        }}
                      >
                        {/* 活动图标 */}
                        <div className="mt-0.5 shrink-0">
                          {getActivityIcon(activity)}
                        </div>

                        {/* 活动信息 */}
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-2 mb-1">
                            <span className="font-medium text-sm">
                              {activity.userName}
                            </span>
                            <Badge
                              variant="outline"
                              className={`text-xs ${getActivityColor(activity)} text-white`}
                            >
                              {getActivityBadge(activity)}
                            </Badge>
                            <span className="text-xs text-muted-foreground ml-auto">
                              {formatTime(new Date(activity.timestamp))}
                            </span>
                          </div>

                          {/* 活动描述 */}
                          <div className="text-sm text-muted-foreground">
                            {activity.description}
                          </div>

                          {/* 文件路径 */}
                          <div className="text-xs text-muted-foreground truncate mt-1">
                            {activity.filePath}
                          </div>
                        </div>

                        {/* 时间差 */}
                        <div className="text-xs text-muted-foreground shrink-0">
                          {formatDate(new Date(activity.timestamp))}
                        </div>
                      </div>
                    ))}
                  </div>
                </div>
              ))}
            </div>
          )}
        </ScrollArea>

        {/* 提示信息 */}
        <div className="flex items-center justify-between text-xs text-muted-foreground border-t pt-4 mt-4">
          <span>{filteredActivities.length} 条记录</span>
          <div className="flex items-center gap-4">
            <span>显示最近活动</span>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
