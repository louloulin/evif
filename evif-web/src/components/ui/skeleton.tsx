import React from 'react'
import { cn } from '@/lib/utils'

interface SkeletonProps extends React.HTMLAttributes<HTMLDivElement> {
  variant?: 'text' | 'circular' | 'rectangular' | 'rounded'
  width?: string | number
  height?: string | number
  animation?: 'pulse' | 'wave' | 'none'
}

export const Skeleton = React.forwardRef<HTMLDivElement, SkeletonProps>(
  (
    {
      className,
      variant = 'rectangular',
      width,
      height,
      animation = 'pulse',
      ...props
    },
    ref
  ) => {
    const variantClasses = {
      text: 'rounded',
      circular: 'rounded-full',
      rectangular: 'rounded-none',
      rounded: 'rounded-md',
    }

    const animationClasses = {
      pulse: 'animate-pulse',
      wave: 'animate-shimmer',
      none: '',
    }

    const style: React.CSSProperties = {
      ...props.style,
      ...(width && { width: typeof width === 'number' ? `${width}px` : width }),
      ...(height && { height: typeof height === 'number' ? `${height}px` : height }),
    }

    return (
      <div
        ref={ref}
        className={cn(
          'bg-muted',
          variantClasses[variant],
          animationClasses[animation],
          className
        )}
        style={style}
        {...props}
      />
    )
  }
)

Skeleton.displayName = 'Skeleton'

// 骨架屏组合组件
interface SkeletonListProps {
  count?: number
  className?: string
}

export const SkeletonText: React.FC<SkeletonProps> = (props) => {
  return <Skeleton variant="text" {...props} />
}

export const SkeletonAvatar: React.FC<SkeletonProps> = (props) => {
  return (
    <Skeleton
      variant="circular"
      width={props.width || 40}
      height={props.height || 40}
      {...props}
    />
  )
}

export const SkeletonCard: React.FC<SkeletonProps> = (props) => {
  return (
    <div className={props.className} style={props.style}>
      <SkeletonText className="mb-2 h-5 w-3/4" />
      <SkeletonText className="h-4 w-1/2" />
    </div>
  )
}

export const SkeletonFileTree: React.FC<SkeletonListProps> = ({
  count = 5,
  className,
}) => {
  return (
    <div className={cn('space-y-2 p-2', className)}>
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="flex items-center gap-2">
          <Skeleton variant="rectangular" width={16} height={16} />
          <SkeletonText className="flex-1" height={32} />
        </div>
      ))}
    </div>
  )
}

export const SkeletonEditor: React.FC<SkeletonProps> = (props) => {
  return (
    <div className={props.className} style={props.style}>
      {/* 工具栏骨架 */}
      <div className="flex items-center gap-2 mb-4 p-2 border-b">
        <Skeleton variant="rectangular" width={20} height={20} />
        <SkeletonText className="flex-1" height={24} />
        <Skeleton variant="rectangular" width={60} height={24} />
      </div>
      {/* 编辑器内容骨架 */}
      <div className="space-y-2 p-4">
        {Array.from({ length: 15 }).map((_, i) => (
          <SkeletonText key={i} height={16} width={`${60 + Math.random() * 40}%`} />
        ))}
      </div>
    </div>
  )
}

export const SkeletonTreeItem: React.FC<{ hasChildren?: boolean }> = ({
  hasChildren = false,
}) => {
  return (
    <div className="flex items-center gap-2 py-1 px-2">
      {hasChildren && <Skeleton variant="rectangular" width={16} height={16} />}
      <Skeleton variant="circular" width={16} height={16} />
      <SkeletonText className="flex-1" height={24} />
    </div>
  )
}
