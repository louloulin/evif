import React, { useState } from 'react'
import { Search, User, Users, Check, X, Mail } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Card, CardContent } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'

interface User {
  id: string
  name: string
  email: string
  avatar?: string
}

interface UserSelectorProps {
  users: User[]
  selectedUsers: User[]
  onSelectionChange: (users: User[]) => void
  placeholder?: string
  maxSelections?: number
}

export const UserSelector: React.FC<UserSelectorProps> = ({
  users,
  selectedUsers,
  onSelectionChange,
  placeholder = '搜索用户...',
  maxSelections,
}) => {
  const [searchQuery, setSearchQuery] = useState('')
  const [isOpen, setIsOpen] = useState(false)

  const filteredUsers = React.useMemo(() => {
    if (!searchQuery.trim()) return users

    const query = searchQuery.toLowerCase()
    return users.filter(
      (user) =>
        user.name.toLowerCase().includes(query) ||
        user.email.toLowerCase().includes(query)
    )
  }, [users, searchQuery])

  const toggleUser = (user: User) => {
    const isSelected = selectedUsers.some((u) => u.id === user.id)

    if (isSelected) {
      // Remove user
      const updated = selectedUsers.filter((u) => u.id !== user.id)
      onSelectionChange(updated)
    } else {
      // Check max selections
      if (maxSelections && selectedUsers.length >= maxSelections) {
        return
      }

      // Add user
      onSelectionChange([...selectedUsers, user])
    }
  }

  const selectAll = () => {
    const count = maxSelections
      ? Math.min(maxSelections, filteredUsers.length)
      : filteredUsers.length

    const selected = filteredUsers.slice(0, count)
    onSelectionChange(selected)
  }

  const clearSelection = () => {
    onSelectionChange([])
  }

  return (
    <Card className="w-full">
      <CardContent className="p-4">
        {/* 搜索框 */}
        <div className="relative mb-4">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={placeholder}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onFocus={() => setIsOpen(true)}
            className="pl-10"
          />
          {searchQuery && (
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setSearchQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 h-6 w-6 p-0"
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>

        {/* 操作按钮 */}
        {selectedUsers.length > 0 && (
          <div className="flex items-center gap-2 mb-4 p-3 bg-muted/50 rounded-lg">
            <Users className="h-4 w-4 text-muted-foreground" />
            <span className="text-sm">
              已选择 {selectedUsers.length} 位用户
            </span>
            {typeof maxSelections === 'number' && (
              <span className="text-xs text-muted-foreground">
                / {maxSelections}
              </span>
            )}
            <div className="flex-1" />
            <Button variant="outline" size="sm" onClick={clearSelection}>
              清除选择
            </Button>
          </div>
        )}

        {/* 用户列表 */}
        <ScrollArea className="h-[400px]">
          {filteredUsers.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <Search className="h-12 w-12 text-muted-foreground mb-3" />
              <p className="text-sm text-muted-foreground">
                {searchQuery ? '未找到匹配的用户' : '暂无用户'}
              </p>
            </div>
          ) : (
            <div className="space-y-2 pr-4">
              {filteredUsers.map((user) => {
                const isSelected = selectedUsers.some((u) => u.id === user.id)

                return (
                  <div
                    key={user.id}
                    onClick={() => toggleUser(user)}
                    className={`
                      flex items-center gap-4 p-3 rounded-lg border cursor-pointer
                      transition-all hover:bg-accent/50
                      ${isSelected ? 'bg-primary/10 border-primary' : 'bg-card border-border'}
                    `}
                  >
                    {/* 选择状态 */}
                    <div
                      className={`
                        flex items-center justify-center h-5 w-5 rounded-full border-2
                        transition-all
                        ${isSelected
                          ? 'bg-primary border-primary'
                          : 'border-muted-foreground'
                        }
                      `}
                    >
                      {isSelected && <Check className="h-3 w-3 text-primary-foreground" />}
                    </div>

                    {/* 用户头像 */}
                    {user.avatar ? (
                      <img
                        src={user.avatar}
                        alt={user.name}
                        className="h-10 w-10 rounded-full"
                      />
                    ) : (
                      <div className="flex items-center justify-center h-10 w-10 rounded-full bg-primary/10">
                        <User className="h-5 w-5 text-primary" />
                      </div>
                    )}

                    {/* 用户信息 */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium truncate">{user.name}</span>
                        {isSelected && (
                          <Badge variant="secondary" className="text-xs">
                            已选择
                          </Badge>
                        )}
                      </div>
                      <div className="flex items-center gap-1 text-sm text-muted-foreground">
                        <Mail className="h-3 w-3" />
                        <span className="truncate">{user.email}</span>
                      </div>
                    </div>
                  </div>
                )
              })}
            </div>
          )}
        </ScrollArea>

        {/* 底部提示 */}
        {filteredUsers.length > 0 && (
          <div className="flex items-center justify-between mt-4 pt-4 border-t text-sm text-muted-foreground">
            <span>共 {filteredUsers.length} 位用户</span>
            {typeof maxSelections === 'number' && (
              <span>
                最多选择 {maxSelections} 位
              </span>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
