import React, { useState } from 'react'
import { MessageSquare, Reply, Send, Check, X, MoreVertical } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent } from '@/components/ui/card'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Textarea } from '@/components/ui/textarea'
import { Comment } from '@/types/collaboration'

interface CommentPanelProps {
  filePath: string
  comments: Comment[]
  onAddComment: (content: string, lineNumber?: number, column?: number, replyTo?: string) => void
  onResolveComment: (commentId: string) => void
  onDeleteComment: (commentId: string) => void
  currentUser?: string
  showResolved?: boolean
}

export const CommentPanel: React.FC<CommentPanelProps> = ({
  filePath,
  comments,
  onAddComment,
  onResolveComment,
  onDeleteComment,
  currentUser = 'current-user',
  showResolved = true,
}) => {
  const [newComment, setNewComment] = useState('')
  const [replyTo, setReplyTo] = useState<string | null>(null)
  const [replyContent, setReplyContent] = useState('')
  const [expandedComment, setExpandedComment] = useState<string | null>(null)

  // 按评论分组 (主评论和回复)
  const groupedComments = React.useMemo(() => {
    const mainComments = comments.filter(c => !c.replyTo)
    return mainComments.map((main) => {
      const replies = comments.filter(c => c.replyTo === main.id)
      return { main, replies }
    })
  }, [comments])

  const handleSubmit = () => {
    if (!newComment.trim()) return

    onAddComment(newComment.trim())
    setNewComment('')
  }

  const handleReply = () => {
    if (!replyContent.trim() || !replyTo) return

    onAddComment(replyContent.trim(), undefined, undefined, replyTo)
    setReplyContent('')
    setReplyTo(null)
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

  return (
    <Card>
      <CardContent className="p-6">
        {/* 评论标题 */}
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold flex items-center gap-2">
            <MessageSquare className="h-5 w-5" />
            文件评论
          </h3>
          <div className="text-sm text-muted-foreground">
            {comments.length} 条评论
          </div>
        </div>

        {/* 评论列表 */}
        <ScrollArea className="h-[500px]">
          {groupedComments.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              暂无评论，添加第一条评论吧！
            </div>
          ) : (
            <div className="space-y-4 pr-4">
              {groupedComments.map(({ main, replies }) => {
                const isResolved = main.resolved
                if (!showResolved && isResolved) return null

                return (
                  <div key={main.id} className="space-y-2">
                    {/* 主评论 */}
                    <Card
                      className={`
                        p-4 transition-all
                        ${isResolved ? 'bg-muted/50 opacity-70' : 'bg-background border'}
                      `}
                    >
                      <CardContent className="space-y-4">
                        {/* 评论头部 */}
                        <div className="flex items-start justify-between gap-4">
                          <div className="flex-1 min-w-0">
                            <div className="flex items-center gap-2 mb-1">
                              <span className="font-medium">{main.author}</span>
                              {main.lineNumber && (
                                <span className="text-sm text-muted-foreground">
                                  行 {main.lineNumber}
                                </span>
                              )}
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {formatDate(new Date(main.createdAt))}
                            </div>
                          </div>

                          {/* 操作按钮 */}
                          {!isResolved && (
                            <div className="flex items-center gap-1 shrink-0">
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => onResolveComment(main.id)}
                                title="标记为已解决"
                              >
                                <Check className="h-4 w-4" />
                              </Button>
                              <Button
                                variant="ghost"
                                size="sm"
                                onClick={() => onDeleteComment(main.id)}
                                title="删除评论"
                              >
                                <X className="h-4 w-4" />
                              </Button>
                            </div>
                          )}
                        </div>

                        {/* 评论内容 */}
                        <div className="text-sm whitespace-pre-wrap">
                          {main.content}
                        </div>

                        {/* 回复输入框 */}
                        {replyTo === main.id && (
                          <div className="flex gap-2 mt-3">
                            <Textarea
                              placeholder="写下你的回复..."
                              value={replyContent}
                              onChange={(e) => setReplyContent(e.target.value)}
                              onKeyDown={(e) => {
                                if (e.key === 'Enter' && !e.shiftKey) {
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
                                setReplyTo(null)
                                setReplyContent('')
                              }}
                            >
                              <X className="h-4 w-4" />
                            </Button>
                          </div>
                        )}

                        {/* 回复按钮 */}
                        {!replyTo && !isResolved && (
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => {
                              setReplyTo(main.id)
                              setReplyContent('')
                            }}
                            className="mt-2"
                          >
                            <Reply className="h-4 w-4 mr-2" />
                            回复
                          </Button>
                        )}
                      </CardContent>
                    </Card>

                    {/* 回复列表 */}
                    {replies.length > 0 && (
                      <div className="ml-8 space-y-2">
                        {replies.map((reply) => (
                          <Card
                            key={reply.id}
                            className="p-4 bg-muted/30 border"
                          >
                            <CardContent className="space-y-2">
                              {/* 回复头部 */}
                              <div className="flex items-start gap-4">
                                <div className="flex-1 min-w-0">
                                  <div className="font-medium text-sm">
                                    {reply.author}
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
                            </CardContent>
                          </Card>
                        ))}
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          )}
        </ScrollArea>

        {/* 添加新评论 */}
        <div className="mt-4 pt-4 border-t space-y-4">
          <div className="flex items-center gap-2">
            <span className="text-sm font-medium">
              {replyTo ? '回复' : '添加评论'}
            </span>
            {replyTo && (
              <span className="text-sm text-muted-foreground">
                @{comments.find(c => c.id === replyTo)?.author}
              </span>
            )}
          </div>
          <Textarea
            placeholder={
              replyTo
                ? '写下你的回复...'
                : '分享你的想法...'
            }
            value={newComment}
            onChange={(e) => setNewComment(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
                e.preventDefault()
                if (replyTo) {
                  handleReply()
                } else {
                  handleSubmit()
                }
              }
            }}
            className="min-h-[100px]"
          />
          <div className="flex items-center gap-2">
            <Button onClick={replyTo ? handleReply : handleSubmit}>
              <Send className="h-4 w-4 mr-2" />
              {replyTo ? '回复' : '发布评论'}
            </Button>
            {replyTo && (
              <Button
                variant="ghost"
                onClick={() => {
                  setReplyTo(null)
                  setNewComment('')
                }}
              >
                取消
              </Button>
            )}
            <span className="text-xs text-muted-foreground ml-auto">
              Ctrl/Cmd + Enter 发送
            </span>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
