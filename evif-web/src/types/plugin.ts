export interface Plugin {
  id: string
  name: string
  version: string
  author: string
  description: string
  type: 'local' | 'cloud-storage' | 'ai' | 'database' | 'other'
  supportTier: 'core' | 'dynamic' | 'experimental' | string
  mountable: boolean
  status: 'loaded' | 'unloaded' | 'error'
  mountPoint?: string
  config?: Record<string, any>
  capabilities: string[]
  dependencies?: string[]
}

export interface PluginConfig {
  [key: string]: any
}

export interface MountPoint {
  path: string
  pluginId: string
  mounted: boolean
  readOnly?: boolean
}
