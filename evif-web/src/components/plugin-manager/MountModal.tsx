import React, { useState } from 'react'
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
import { Plugin, MountPoint } from '@/types/plugin'
import { Loader2, Folder } from 'lucide-react'

interface MountModalProps {
  plugin: Plugin | null
  open: boolean
  onClose: () => void
  onMount: (pluginId: string, mountPoint: string, options?: { readOnly?: boolean }) => Promise<void>
  existingMounts?: MountPoint[]
}

export const MountModal: React.FC<MountModalProps> = ({
  plugin,
  open,
  onClose,
  onMount,
  existingMounts = [],
}) => {
  const [mountPath, setMountPath] = useState('')
  const [readOnly, setReadOnly] = useState(false)
  const [mounting, setMounting] = useState(false)

  if (!plugin) return null

  const handleMount = async () => {
    if (!mountPath.trim()) return

    setMounting(true)
    try {
      await onMount(plugin.id, mountPath, { readOnly })
      onClose()
      setMountPath('')
      setReadOnly(false)
    } finally {
      setMounting(false)
    }
  }

  const isPathTaken = existingMounts.some(m => m.path === mountPath)

  return (
    <Dialog open={open} onOpenChange={onClose}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>Mount {plugin.name}</DialogTitle>
          <DialogDescription>
            Choose a mount point for this plugin. This will create a virtual filesystem path.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <div className="space-y-2">
            <Label htmlFor="mount-path">
              <Folder className="inline h-4 w-4 mr-2" />
              Mount Path
            </Label>
            <Input
              id="mount-path"
              placeholder="/mnt/plugin-name"
              value={mountPath}
              onChange={(e) => setMountPath(e.target.value)}
              className={isPathTaken ? 'border-red-500' : ''}
            />
            {isPathTaken && (
              <p className="text-sm text-red-500">
                This path is already mounted
              </p>
            )}
            <p className="text-xs text-muted-foreground">
              Enter an absolute path where this plugin will be mounted
            </p>
          </div>

          <div className="flex items-center space-x-2">
            <input
              type="checkbox"
              id="readonly"
              checked={readOnly}
              onChange={(e) => setReadOnly(e.target.checked)}
              className="rounded"
            />
            <Label htmlFor="readonly" className="cursor-pointer">
              Mount as read-only
            </Label>
          </div>

          {/* Existing Mounts Info */}
          {existingMounts.length > 0 && (
            <div className="border rounded-md p-3">
              <p className="text-sm font-medium mb-2">Current Mounts:</p>
              <div className="space-y-1">
                {existingMounts.map((mount, i) => (
                  <div key={i} className="text-xs text-muted-foreground flex items-center gap-2">
                    <Folder className="h-3 w-3" />
                    <span>{mount.path}</span>
                    {mount.readOnly && <span className="text-xs">(read-only)</span>}
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={onClose} disabled={mounting}>
            Cancel
          </Button>
          <Button
            onClick={handleMount}
            disabled={mounting || !mountPath.trim() || isPathTaken}
          >
            {mounting && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            Mount Plugin
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

export default MountModal
