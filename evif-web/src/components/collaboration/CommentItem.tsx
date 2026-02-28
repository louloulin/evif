import React from 'react'
import { User, Clock, MoreVertical, Check, X, Trash2 } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Comment } from '@/types/collaboration'

interface CommentItemProps {
  comment: Comment
  isReply?: boolean
  isMain?: boolean
  currentUser?: string
  onReply?: (commentId: string) => void
  onResolve?: (commentId: string) => void
  onDelete?: (commentId: string) => void
  isResolved?: boolean
  isReplyingTo?: boolean
}

export const CommentItem: React.FC<CommentItemProps> = ({
  comment,
  isReply = false,
  isMain = false,
  currentUser,
  onReply,
  onResolve,
  onDelete,
  isResolved = false,
  isReplyingTo = false,
}) => {
  const isCurrentUser = comment.authorId === currentUser

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

  return (
    <Card
      className={`
        p-4 transition-all
        ${isReply ? 'bg-muted/30 border' : 'bg-background border'}
        ${isResolved ? 'bg-muted/50 opacity-70' : ''}
      `}
    >
      <CardContent className="space-y-4">
        {/* 评论头部 */}
        <div className="flex items-start justify-between gap-4">
          <div className="flex-1 min-w-0">
            {/* 作者信息 */}
            <div className="flex items-center gap-2 mb-1">
              <span className="font-medium">{comment.author}</span>
              {isCurrentUser && (
                <Badge variant="secondary" className="text-xs">
                  我
                </Badge>
              )}
              {isReply && (
                <Badge variant="outline" className="text-xs">
                  回复
                </Badge>
              )}
            </div>

            {/* 行号信息 */}
            {comment.lineNumber && (
              <div className="text-sm text-muted-foreground">
                行 {comment.lineNumber}
                {comment.column && ` 列 ${comment.column}`}
              </div>
            )}

            {/* 时间信息 */}
            <div className="text-xs text-muted-foreground">
              {formatDate(new Date(comment.createdAt))}
              <span className="mx-1">•</span>
              {formatTime(new Date(comment.createdAt))}
            </div>
          </div>

          {/* 操作按钮 */}
          {!isReply && !isResolved && (
            <div className="flex items-center gap-1 shrink-0">
              {onResolve && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onResolve(comment.id)}
                  title="标记为已解决"
                >
                  <Check className="h-4 w-4" />
                </Button>
              )}
              {onDelete && (
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => onDelete(comment.id)}
                  title="删除评论"
                >
                  <Trash2 className="h-4 w-4" />
                </Button>
              )}
            </div>
          )}
        </div>

        {/* 评论内容 */}
        <div className="text-sm whitespace-pre-wrap bg-muted/20 rounded p-3">
          {comment.content}
        </div>

        {/* 元数据 */}
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <User className="h-3 w-3" />
          <span>{comment.authorId}</span>
          {isReply && comment.replyTo && (
            <>
              <span>•</span>
              <span>回复 @{comment.replyTo}</span>
            </>
          )}
        </div>

        {/* 回复按钮 */}
        {!isResolved && onReply && !isReplyingTo && (
          <Button
            variant="ghost"
            size="sm"
            onClick={() => onReply(comment.id)}
            className="mt-2"
          >
            回复
          </Button>
        )}
      </CardContent>
    </Card>
  )
}
