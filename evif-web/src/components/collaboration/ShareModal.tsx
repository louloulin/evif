import React, { useState } from 'react'
import { Share2, Copy, Link, Calendar, Shield, UserPlus, Globe } from 'lucide-react'
import { Dialog, DialogContent, DialogHeader, DialogTitle, DialogTrigger } from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Badge } from '@/components/ui/badge'
import { Switch } from '@/components/ui/switch'
import { Share, Permission } from '@/types/collaboration'

interface ShareModalProps {
  open: boolean
  onClose: () => void
  filePath: string
  fileName: string
  onShare: (share: Omit<Share, 'id' | 'createdBy' | 'createdAt'>) => void
}

export const ShareModal: React.FC<ShareModalProps> = ({
  open,
  onClose,
  filePath,
  fileName,
  onShare,
}) => {
  const [accessType, setAccessType] = useState<'anyone' | 'specific' | 'link'>('link')
  const [expiry, setExpiry] = useState<'never' | '1hour' | '1day' | '1week' | 'custom'>('never')
  const [customExpiry, setCustomExpiry] = useState('')
  const [permissions, setPermissions] = useState<Permission[]>(['read'])
  const [copied, setCopied] = useState(false)

  const handleCopyLink = () => {
    setCopied(true)
    setTimeout(() => setCopied(false), 2000)
  }

  const handleShare = () => {
    onShare({
      fileId: 'file-123',
      fileName,
      filePath,
      expiresAt:
        expiry === 'never'
          ? undefined
          : expiry === 'custom'
          ? new Date(customExpiry)
          : new Date(Date.now() + getExpiryMs(expiry)),
      accessUrl: `https://evif.example.com/share/${generateShareId()}`,
      permissions: [],
      accessCount: 0,
    })
    onClose()
  }

  const generateShareId = (): string => {
    return Math.random().toString(36).substring(2, 10)
  }

  const getExpiryMs = (expiry: string): number => {
    switch (expiry) {
      case '1hour':
        return 60 * 60 * 1000
      case '1day':
        return 24 * 60 * 60 * 1000
      case '1week':
        return 7 * 24 * 60 * 60 * 1000
      default:
        return 0
    }
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Share2 className="h-5 w-5" />
            分享文件
          </DialogTitle>
        </DialogHeader>

        <div className="space-y-6">
          {/* 文件信息 */}
          <Card>
            <CardContent className="flex items-center gap-4 py-3">
              <Link className="h-10 w-10 text-muted-foreground" />
              <div className="flex-1 min-w-0">
                <div className="font-medium">{fileName}</div>
                <div className="text-sm text-muted-foreground truncate">
                  {filePath}
                </div>
              </div>
            </CardContent>
          </Card>

          {/* 访问方式 */}
          <div className="space-y-4">
            <Label>访问方式</Label>
            <div className="flex gap-2">
              <Button
                variant={accessType === 'link' ? 'default' : 'outline'}
                className="flex-1"
                onClick={() => setAccessType('link')}
              >
                <Globe className="h-4 w-4 mr-2" />
                链接
              </Button>
              <Button
                variant={accessType === 'anyone' ? 'default' : 'outline'}
                className="flex-1"
                onClick={() => setAccessType('anyone')}
              >
                <UserPlus className="h-4 w-4 mr-2" />
                任何人
              </Button>
              <Button
                variant={accessType === 'specific' ? 'default' : 'outline'}
                className="flex-1"
                onClick={() => setAccessType('specific')}
              >
                <Shield className="h-4 w-4 mr-2" />
                指定用户
              </Button>
            </div>
          </div>

          {/* 权限设置 */}
          <Card>
            <CardHeader>
              <CardTitle className="text-sm font-medium">权限设置</CardTitle>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center justify-between">
                <Label htmlFor="perm-read" className="flex items-center gap-2">
                  <Shield className="h-4 w-4 text-blue-500" />
                  读取
                </Label>
                <Switch
                  id="perm-read"
                  checked={permissions.includes('read')}
                  onCheckedChange={(checked: boolean) => {
                    if (checked && !permissions.includes('read')) {
                      setPermissions([...permissions, 'read'])
                    } else if (!checked) {
                      setPermissions(permissions.filter(p => p !== 'read'))
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between">
                <Label htmlFor="perm-write" className="flex items-center gap-2">
                  <Shield className="h-4 w-4 text-yellow-500" />
                  写入
                </Label>
                <Switch
                  id="perm-write"
                  checked={permissions.includes('write')}
                  onCheckedChange={(checked: boolean) => {
                    if (checked && !permissions.includes('write')) {
                      setPermissions([...permissions, 'write'])
                    } else if (!checked) {
                      setPermissions(permissions.filter(p => p !== 'write'))
                    }
                  }}
                />
              </div>

              <div className="flex items-center justify-between">
                <Label htmlFor="perm-execute" className="flex items-center gap-2">
                  <Shield className="h-4 w-4 text-green-500" />
                  执行
                </Label>
                <Switch
                  id="perm-execute"
                  checked={permissions.includes('execute')}
                  onCheckedChange={(checked: boolean) => {
                    if (checked && !permissions.includes('execute')) {
                      setPermissions([...permissions, 'execute'])
                    } else if (!checked) {
                      setPermissions(permissions.filter(p => p !== 'execute'))
                    }
                  }}
                />
              </div>
            </CardContent>
          </Card>

          {/* 过期时间 */}
          <div className="space-y-4">
            <Label>过期时间</Label>
            <div className="flex gap-2">
              <Button
                variant={expiry === 'never' ? 'default' : 'outline'}
                onClick={() => setExpiry('never')}
              >
                永久
              </Button>
              <Button
                variant={expiry === '1hour' ? 'default' : 'outline'}
                onClick={() => setExpiry('1hour')}
              >
                1 小时
              </Button>
              <Button
                variant={expiry === '1day' ? 'default' : 'outline'}
                onClick={() => setExpiry('1day')}
              >
                1 天
              </Button>
              <Button
                variant={expiry === '1week' ? 'default' : 'outline'}
                onClick={() => setExpiry('1week')}
              >
                1 周
              </Button>
              <Button
                variant={expiry === 'custom' ? 'default' : 'outline'}
                onClick={() => setExpiry('custom')}
              >
                自定义
              </Button>
            </div>
            {expiry === 'custom' && (
              <Input
                type="datetime-local"
                value={customExpiry}
                onChange={(e) => setCustomExpiry(e.target.value)}
                className="mt-2"
              />
            )}
          </div>

          {/* 分享按钮 */}
          <Button
            onClick={handleShare}
            disabled={permissions.length === 0}
            className="w-full"
          >
            <Share2 className="h-4 w-4 mr-2" />
            生成分享链接
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  )
}
