import React, { useState } from 'react'
import { MessageSquare, ChevronDown, ChevronRight, Reply, Send } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Textarea } from '@/components/ui/textarea'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Badge } from '@/components/ui/badge'
import { Comment } from '@/types/collaboration'

interface ThreadViewProps {
  comments: Comment[]
  mainCommentId: string
  currentUser?: string
  onAddReply: (content: string, replyTo: string) => void
  onResolveComment: (commentId: string) => void
  onDeleteComment: (commentId: string) => void
}

export const ThreadView: React.FC<ThreadViewProps> = ({
  comments,
  mainCommentId,
  currentUser,
  onAddReply,
  onResolveComment,
  onDeleteComment,
}) => {
  const [expanded, setExpanded] = useState(true)
  const [replyContent, setReplyContent] = useState('')
  const [replyingTo, setReplyingTo] = useState<string | null>(null)

  // 获取主评论
  const mainComment = React.useMemo(
    () => comments.find((c) => c.id === mainCommentId),
    [comments, mainCommentId]
  )

  // 获取回复列表
  const replies = React.useMemo(
    () => comments.filter((c) => c.replyTo === mainCommentId),
    [comments, mainCommentId]
  )

  const handleReply = () => {
    if (!replyContent.trim() || !replyingTo) return

    onAddReply(replyContent.trim(), replyingTo)
    setReplyContent('')
    setReplyingTo(null)
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

  if (!mainComment) {
    return <div>评论不存在</div>
  }

  const isResolved = mainComment.resolved
  const isCurrentUser = mainComment.authorId === currentUser

  return (
    <Card
      className={`
        transition-all
        ${isResolved ? 'bg-muted/50 opacity-70' : 'bg-background border'}
      `}
    >
      {/* 主评论头部（可折叠） */}
      <CardHeader
        className="cursor-pointer hover:bg-accent/50 transition-colors"
        onClick={() => setExpanded(!expanded)}
      >
        <div className="flex items-center gap-4">
          <div className="shrink-0">
            {expanded ? (
              <ChevronDown className="h-4 w-4 text-muted-foreground" />
            ) : (
              <ChevronRight className="h-4 w-4 text-muted-foreground" />
            )}
          </div>

          <CardTitle className="flex-1 flex items-center gap-2">
            <MessageSquare className="h-4 w-4" />
            <span>{mainComment.author}</span>
            {isResolved && (
              <Badge variant="secondary" className="text-xs">
                已解决
              </Badge>
            )}
          </CardTitle>

          <div className="flex items-center gap-2 text-sm text-muted-foreground">
            <span>{replies.length} 条回复</span>
            <span>•</span>
            <span>{formatDate(new Date(mainComment.createdAt))}</span>
          </div>
        </div>
      </CardHeader>

      {/* 主评论内容 */}
      {expanded && (
        <CardContent className="space-y-4">
          {/* 行号信息 */}
          {mainComment.lineNumber && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span>行 {mainComment.lineNumber}</span>
              {mainComment.column && <span> 列 {mainComment.column}</span>}
            </div>
          )}

          {/* 评论内容 */}
          <div className="text-sm whitespace-pre-wrap bg-muted/20 rounded-lg p-4">
            {mainComment.content}
          </div>

          {/* 操作按钮 */}
          {!isResolved && (
            <div className="flex items-center justify-between">
              <div className="flex items-center gap-2">
                {onResolveComment && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => onResolveComment(mainComment.id)}
                  >
                    标记为已解决
                  </Button>
                )}
              </div>

              <div className="flex items-center gap-2">
                {onDeleteComment && (
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => onDeleteComment(mainComment.id)}
                  >
                    删除
                  </Button>
                )}
              </div>
            </div>
          )}

          {/* 回复列表 */}
          {replies.length > 0 && (
            <div className="border-t pt-4">
              <div className="text-sm font-medium mb-3">
                回复 ({replies.length})
              </div>
              <ScrollArea className="h-[300px]">
                <div className="space-y-4 pr-4">
                  {replies.map((reply) => {
                    const isReplyingToThis = replyingTo === reply.id

                    return (
                      <div
                        key={reply.id}
                        className="bg-muted/30 rounded-lg p-4 border"
                      >
                        {/* 回复头部 */}
                        <div className="flex items-start justify-between gap-4 mb-2">
                          <div className="flex-1">
                            <div className="flex items-center gap-2">
                              <span className="font-medium">{reply.author}</span>
                              {reply.authorId === currentUser && (
                                <Badge variant="secondary" className="text-xs">
                                  我
                                </Badge>
                              )}
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {formatDate(new Date(reply.createdAt))}
                            </div>
                          </div>
                        </div>

                        {/* 回复内容 */}
                        <div className="text-sm whitespace-pre-wrap">
                          {reply.content}
                        </div>

                        {/* 回复输入框 */}
                        {isReplyingToThis && (
                          <div className="mt-3 flex gap-2">
                            <Textarea
                              placeholder="写下你的回复..."
                              value={replyContent}
                              onChange={(e) => setReplyContent(e.target.value)}
                              onKeyDown={(e) => {
                                if (
                                  e.key === 'Enter' &&
                                  !e.shiftKey &&
                                  !e.metaKey &&
                                  !e.ctrlKey
                                ) {
                                  e.preventDefault()
                                  handleReply()
                                }
                              }}
                              className="flex-1 min-h-[80px]"
                            />
                            <Button size="sm" onClick={handleReply}>
                              <Send className="h-4 w-4" />
                            </Button>
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => {
                                setReplyingTo(null)
                                setReplyContent('')
                              }}
                            >
                              取消
                            </Button>
                          </div>
                        )}
                      </div>
                    )
                  })}
                </div>
              </ScrollArea>
            </div>
          )}

          {/* 添加回复按钮 */}
          {!isResolved && !replyingTo && (
            <div className="border-t pt-4">
              <Button
                variant="outline"
                className="w-full"
                onClick={() => {
                  setReplyingTo(mainComment.id)
                  setReplyContent('')
                }}
              >
                <Reply className="h-4 w-4 mr-2" />
                回复此评论
              </Button>
            </div>
          )}
        </CardContent>
      )}
    </Card>
  )
}
