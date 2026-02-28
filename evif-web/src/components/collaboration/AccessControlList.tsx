import React from 'react'
import { Shield, Lock, ChevronRight, ChevronDown } from 'lucide-react'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { ScrollArea } from '@/components/ui/scroll-area'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { Permission } from '@/types/collaboration'

interface AccessEntry {
  id: string
  type: 'user' | 'group' | 'public'
  name: string
  email?: string
  permissions: Permission[]
  path: string
  inheritedFrom?: string
}

interface AccessControlListProps {
  entries: AccessEntry[]
  filePath: string
  onTogglePermission: (entryId: string, permission: Permission) => void
  onRemoveEntry: (entryId: string) => void
  showInheritance?: boolean
}

export const AccessControlList: React.FC<AccessControlListProps> = ({
  entries,
  filePath,
  onTogglePermission,
  onRemoveEntry,
  showInheritance = true,
}) => {
  const [expandedEntries, setExpandedEntries] = React.useState<Set<string>>(new Set())

  const toggleExpand = (entryId: string) => {
    const newExpanded = new Set(expandedEntries)
    if (newExpanded.has(entryId)) {
      newExpanded.delete(entryId)
    } else {
      newExpanded.add(entryId)
    }
    setExpandedEntries(newExpanded)
  }

  const getTypeBadge = (entry: AccessEntry): React.ReactNode => {
    const badges: Record<AccessEntry['type'], React.ReactNode> = {
      user: (
        <Badge variant="outline" className="bg-blue-50 text-blue-700 border-blue-200">用户</Badge>
      ),
      group: (
        <Badge variant="outline" className="bg-purple-50 text-purple-700 border-purple-200">群组</Badge>
      ),
      public: (
        <Badge variant="outline" className="bg-green-50 text-green-700 border-green-200">公开</Badge>
      ),
    }
    return badges[entry.type]
  }

  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <CardTitle className="flex items-center gap-2">
            <Shield className="h-5 w-5" />
            访问控制列表
          </CardTitle>
          <Badge variant="secondary">{entries.length} 条规则</Badge>
        </div>
        <div className="text-sm text-muted-foreground mt-2">{filePath}</div>
      </CardHeader>

      <CardContent>
        <ScrollArea className="h-[500px]">
          {entries.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-center">
              <Shield className="h-12 w-12 text-muted-foreground mb-3" />
              <p className="text-sm text-muted-foreground">暂无访问控制规则</p>
            </div>
          ) : (
            <div className="space-y-4 pr-4">
              {entries.map((entry) => {
                const isExpanded = expandedEntries.has(entry.id)
                return (
                  <div key={entry.id} className="border rounded-lg overflow-hidden hover:border-primary/50 transition-colors">
                    <div className="flex items-center gap-4 p-4 bg-card cursor-pointer hover:bg-accent/50 transition-colors" onClick={() => toggleExpand(entry.id)}>
                      <div className="shrink-0">
                        {isExpanded ? <ChevronDown className="h-4 w-4 text-muted-foreground" /> : <ChevronRight className="h-4 w-4 text-muted-foreground" />}
                      </div>
                      {getTypeBadge(entry)}
                      <div className="flex-1 min-w-0">
                        <div className="font-medium">{entry.name}</div>
                        {entry.email && <div className="text-sm text-muted-foreground truncate">{entry.email}</div>}
                      </div>
                      <div className="flex items-center gap-2">
                        {entry.permissions.map((perm) => (
                          <span key={perm} className="font-mono font-semibold">{perm[0].toUpperCase()}</span>
                        ))}
                      </div>
                    </div>
                    {isExpanded && (
                      <div className="p-4 border-t bg-muted/30 space-y-4" onClick={(e) => e.stopPropagation()}>
                        <div>
                          <div className="text-xs text-muted-foreground mb-1">路径</div>
                          <div className="text-sm font-mono bg-background rounded p-2">{entry.path}</div>
                        </div>
                        {showInheritance && entry.inheritedFrom && (
                          <div>
                            <div className="text-xs text-muted-foreground mb-1">继承自</div>
                            <div className="text-sm">{entry.inheritedFrom}</div>
                          </div>
                        )}
                        <div>
                          <div className="text-xs text-muted-foreground mb-2">权限设置</div>
                          <div className="flex flex-wrap gap-4">
                            {(['read', 'write', 'execute', 'admin'] as Permission[]).map((perm) => (
                              <div key={perm} className="flex items-center gap-2">
                                <Switch checked={entry.permissions.includes(perm)} onCheckedChange={() => onTogglePermission(entry.id, perm)} />
                                <span className="text-sm">{perm}</span>
                              </div>
                            ))}
                          </div>
                        </div>
                        <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                          <Button variant="destructive" size="sm" onClick={() => onRemoveEntry(entry.id)}>删除规则</Button>
                        </div>
                      </div>
                    )}
                  </div>
                )
              })}
            </div>
          )}
        </ScrollArea>
        <div className="flex items-start gap-4 border-t pt-4 mt-4 text-xs text-muted-foreground">
          <Lock className="h-4 w-4 shrink-0 mt-0.5" />
          <div className="space-y-1">
            <p><strong>权限说明:</strong></p>
            <p>• 读取 (R): 查看文件内容</p>
            <p>• 写入 (W): 修改或删除文件</p>
            <p>• 执行 (X): 运行可执行文件</p>
            <p>• 管理 (A): 完全控制权限</p>
            <p className="mt-2 text-muted-foreground">* 权限可以继承自父目录</p>
          </div>
        </div>
      </CardContent>
    </Card>
  )
}
