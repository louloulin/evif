import React, { useState, useEffect } from 'react'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Plugin, PluginConfig } from '@/types/plugin'
import { Loader2 } from 'lucide-react'

interface PluginModalProps {
  plugin: Plugin | null
  open: boolean
  onClose: () => void
  onSave: (pluginId: string, config: PluginConfig) => Promise<void>
  /** Phase 9.1: 可选，从 API 获取的 README 内容，用于“查看说明” */
  readmeOverride?: string
  /** Phase 9.1: 可选，从 API 获取的配置参数默认值，仅展示用 */
  configParamsOverride?: PluginConfig
}

export const PluginModal: React.FC<PluginModalProps> = ({
  plugin,
  open,
  onClose,
  onSave,
  readmeOverride,
  configParamsOverride,
}) => {
  const [config, setConfig] = useState<PluginConfig>(configParamsOverride ?? plugin?.config ?? {})
  const [saving, setSaving] = useState(false)

  useEffect(() => {
    if (open && plugin) {
      setConfig(configParamsOverride ?? plugin.config ?? {})
    }
  }, [open, plugin?.id, configParamsOverride])

  if (!plugin) return null

  const handleSave = async () => {
    setSaving(true)
    try {
      await onSave(plugin.id, config)
      onClose()
    } finally {
      setSaving(false)
    }
  }

  // Common config fields for different plugin types
  const renderConfigFields = () => {
    switch (plugin.type) {
      case 'cloud-storage':
        return (
          <>
            <div className="space-y-2">
              <Label htmlFor="access-key">访问密钥</Label>
              <Input
                id="access-key"
                type="password"
                value={config.accessKey || ''}
                onChange={(e) => setConfig({ ...config, accessKey: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="secret-key">秘密密钥</Label>
              <Input
                id="secret-key"
                type="password"
                value={config.secretKey || ''}
                onChange={(e) => setConfig({ ...config, secretKey: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="bucket">存储桶/容器</Label>
              <Input
                id="bucket"
                value={config.bucket || ''}
                onChange={(e) => setConfig({ ...config, bucket: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="region">区域</Label>
              <Input
                id="region"
                value={config.region || ''}
                onChange={(e) => setConfig({ ...config, region: e.target.value })}
              />
            </div>
          </>
        )

      case 'database':
        return (
          <>
            <div className="space-y-2">
              <Label htmlFor="connection-string">连接字符串</Label>
              <Input
                id="connection-string"
                type="password"
                value={config.connectionString || ''}
                onChange={(e) => setConfig({ ...config, connectionString: e.target.value })}
                placeholder="postgresql://user:password@localhost:5432/db"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="table">表名</Label>
              <Input
                id="table"
                value={config.table || ''}
                onChange={(e) => setConfig({ ...config, table: e.target.value })}
              />
            </div>
          </>
        )

      case 'ai':
        return (
          <>
            <div className="space-y-2">
              <Label htmlFor="api-key">API密钥</Label>
              <Input
                id="api-key"
                type="password"
                value={config.apiKey || ''}
                onChange={(e) => setConfig({ ...config, apiKey: e.target.value })}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="model">模型</Label>
              <Input
                id="model"
                value={config.model || ''}
                onChange={(e) => setConfig({ ...config, model: e.target.value })}
                placeholder="gpt-4, claude-3, etc."
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="endpoint">端点 (可选)</Label>
              <Input
                id="endpoint"
                value={config.endpoint || ''}
                onChange={(e) => setConfig({ ...config, endpoint: e.target.value })}
              />
            </div>
          </>
        )

      default:
        return (
          <div className="text-sm text-muted-foreground">
            此插件无需配置。
          </div>
        )
    }
  }

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>配置 {plugin.name}</DialogTitle>
          <DialogDescription>
            更新此插件的配置设置。
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {readmeOverride && readmeOverride.length > 0 && (
            <div className="rounded-md border bg-muted/30 p-3 max-h-48 overflow-y-auto">
              <p className="text-xs font-medium text-muted-foreground mb-2">README</p>
              <pre className="text-xs whitespace-pre-wrap font-sans">{readmeOverride}</pre>
            </div>
          )}
          {renderConfigFields()}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={saving}>
            取消
          </Button>
          <Button onClick={handleSave} disabled={saving}>
            {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            保存更改
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default PluginModal
