import React from 'react'
import { ChevronRight, Home } from 'lucide-react'
import { buttonVariants } from '@/components/ui/button'

interface BreadcrumbItem {
  name: string
  path: string
}

interface BreadcrumbProps {
  filePath: string
  onNavigate?: (path: string) => void
}

export const Breadcrumb: React.FC<BreadcrumbProps> = ({
  filePath,
  onNavigate,
}) => {
  // 解析文件路径为面包屑项
  const items = React.useMemo((): BreadcrumbItem[] => {
    if (!filePath) return []

    // 移除开头的 /
    const cleanPath = filePath.startsWith('/') ? filePath.slice(1) : filePath
    const parts = cleanPath.split('/')

    // 构建面包屑项
    const breadcrumbItems: BreadcrumbItem[] = []

    // 添加根目录
    breadcrumbItems.push({
      name: '根目录',
      path: '/',
    })

    // 添加每个目录
    let currentPath = ''
    parts.forEach((part, index) => {
      currentPath += '/' + part

      // 如果是最后一项(文件名),使用文件名
      // 否则使用目录名
      const isLast = index === parts.length - 1
      breadcrumbItems.push({
        name: part,
        path: currentPath,
      })
    })

    return breadcrumbItems
  }, [filePath])

  if (items.length === 0) {
    return null
  }

  const handleItemClick = (item: BreadcrumbItem) => {
    if (onNavigate) {
      onNavigate(item.path)
    }
  }

  return (
    <div className="flex items-center gap-1 px-4 py-2 text-sm border-b bg-muted/30">
      {items.map((item, index) => (
        <React.Fragment key={item.path}>
          {index > 0 && (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          <button
            onClick={() => handleItemClick(item)}
            className={`
              flex items-center gap-1 px-2 py-1 rounded transition-colors
              ${index === items.length - 1
                ? 'text-foreground font-medium cursor-default'
                : buttonVariants({ variant: 'ghost', size: 'sm' })
              }
            `}
            disabled={index === items.length - 1 || !onNavigate}
          >
            {index === 0 && <Home className="h-3.5 w-3.5" />}
            <span className="max-w-[200px] truncate">{item.name}</span>
          </button>
        </React.Fragment>
      ))}
    </div>
  )
}
