import React, { useState } from 'react'
import { Trash2, Edit, Shield, UserPlus, X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Switch } from '@/components/ui/switch'
import { Permission, SharePermission } from '@/types/collaboration'

interface PermissionEditorProps {
  filePath: string
  fileName: string
  existingPermissions?: SharePermission[]
  onUpdate: (permissions: SharePermission[]) => void
}

export const PermissionEditor: React.FC<PermissionEditorProps> = ({
  filePath,
  fileName,
  existingPermissions = [],
  onUpdate,
}) => {
  const [permissions, setPermissions] = useState<SharePermission[]>([
    ...existingPermissions,
  ])
  const [addingUser, setAddingUser] = useState(false)
  const [newUserEmail, setNewUserEmail] = useState('')

  const handleAddUser = () => {
    if (!newUserEmail.trim()) return

    const newPermission: SharePermission = {
      userId: generateUserId(),
      userName: newUserEmail,
      permissions: ['read'],
    }

    setPermissions([...permissions, newPermission])
    setNewUserEmail('')
    setAddingUser(false)
    onUpdate([...permissions, newPermission])
  }

  const handleRemoveUser = (userId: string) => {
    const updated = permissions.filter((p) => p.userId !== userId)
    setPermissions(updated)
    onUpdate(updated)
  }

  const handleTogglePermission = (
    userId: string,
    perm: Permission,
    enabled: boolean
  ) => {
    const updated = permissions.map((p) => {
      if (p.userId === userId) {
        return {
          ...p,
          permissions: enabled
            ? [...p.permissions, perm]
            : p.permissions.filter((p) => p !== perm),
        }
      }
      return p
    })

    setPermissions(updated)
    onUpdate(updated)
  }

  const generateUserId = (): string => {
    return `user-${Date.now()}`
  }

  const getPermissionBadge = (perm: Permission) => {
    const colors: Record<Permission, string> = {
      read: 'bg-blue-500',
      write: 'bg-yellow-500',
      execute: 'bg-green-500',
      admin: 'bg-red-500',
    }

    const icons: Record<Permission, string> = {
      read: 'R',
      write: 'W',
      execute: 'X',
      admin: 'A',
    }

    return (
      <Badge
        variant="outline"
        className={`${colors[perm]} text-white`}
        title={perm}
      >
        {icons[perm]}
      </Badge>
    )
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            权限编辑器
          </CardTitle>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setAddingUser(!addingUser)}
          >
            <UserPlus className="h-4 w-4 mr-2" />
            添加用户
          </Button>
        </div>
        <div className="text-sm text-muted-foreground mt-2">
          {fileName}
        </div>
      </CardHeader>

      <CardContent className="space-y-4">
        {/* 添加用户输入 */}
        {addingUser && (
          <div className="flex gap-2 p-4 border rounded-lg bg-muted/30">
            <Input
              type="email"
              placeholder="输入用户邮箱"
              value={newUserEmail}
              onChange={(e) => setNewUserEmail(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter') {
                  handleAddUser()
                } else if (e.key === 'Escape') {
                  setAddingUser(false)
                  setNewUserEmail('')
                }
              }}
              className="flex-1"
            />
            <Button onClick={handleAddUser}>添加</Button>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => {
                setAddingUser(false)
                setNewUserEmail('')
              }}
            >
              <X className="h-4 w-4" />
            </Button>
          </div>
        )}

        {/* 权限列表 */}
        <ScrollArea className="h-[400px]">
          <div className="space-y-2 pr-4">
            {permissions.length === 0 ? (
              <div className="text-center py-8 text-muted-foreground">
                暂无权限设置
              </div>
            ) : (
              permissions.map((userPerm) => (
                <Card
                  key={userPerm.userId}
                  className="p-4 hover:bg-accent/50 transition-colors"
                >
                  <CardContent className="flex items-center justify-between">
                    {/* 用户信息 */}
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="font-medium">{userPerm.userName}</span>
                        <Badge variant="secondary" className="text-xs">
                          {userPerm.userId}
                        </Badge>
                      </div>
                      <div className="text-xs text-muted-foreground">
                        {filePath}
                      </div>
                    </div>

                    {/* 操作 */}
                    <div className="flex items-center gap-4">
                      {/* 权限开关 */}
                      <div className="flex gap-2">
                        {(['read', 'write', 'execute'] as Permission[]).map(
                          (perm) => (
                            <Switch
                              key={perm}
                              checked={userPerm.permissions.includes(perm)}
                              onCheckedChange={(checked: boolean) =>
                                handleTogglePermission(userPerm.userId, perm, checked)
                              }
                            />
                          )
                        )}
                      </div>

                      {/* 删除按钮 */}
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => handleRemoveUser(userPerm.userId)}
                        title="移除用户"
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </CardContent>
                </Card>
              ))
            )}
          </div>
        </ScrollArea>

        {/* 权限说明 */}
        <div className="text-xs text-muted-foreground border-t pt-4">
          <div className="space-y-1">
            <p><strong>权限说明:</strong></p>
            <p>• 读取 (R): 查看文件内容</p>
            <p>• 写入 (W): 修改或删除文件</p>
            <p>• 执行 (X): 运行可执行文件</p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
