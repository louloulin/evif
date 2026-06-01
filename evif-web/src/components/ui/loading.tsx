/**
 * Unified Loading Components
 * 提供统一的 Loading 状态组件
 */

import React from 'react'
import { Loader2, FileQuestion, Inbox, FolderOpen, Search, AlertCircle } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from './button'

// ============ Loading Spinner ============

interface LoadingSpinnerProps {
  size?: 'sm' | 'md' | 'lg'
  className?: string
}

export function LoadingSpinner({ size = 'md', className }: LoadingSpinnerProps) {
  const sizeClasses = {
    sm: 'h-4 w-4',
    md: 'h-6 w-6',
    lg: 'h-8 w-8',
  }
  
  return (
    <Loader2 className={cn('animate-spin text-muted-foreground', sizeClasses[size], className)} />
  )
}

// ============ Loading Overlay ============

interface LoadingOverlayProps {
  message?: string
}

export function LoadingOverlay({ message = 'Loading...' }: LoadingOverlayProps) {
  return (
    <div className="absolute inset-0 flex items-center justify-center bg-background/80 backdrop-blur-sm z-50">
      <div className="flex flex-col items-center gap-3">
        <LoadingSpinner size="lg" />
        <p className="text-sm text-muted-foreground">{message}</p>
      </div>
    </div>
  )
}

// ============ Skeleton ============

interface SkeletonProps {
  className?: string
}

export function Skeleton({ className }: SkeletonProps) {
  return (
    <div className={cn('animate-pulse rounded-md bg-muted', className)} />
  )
}

// ============ Skeleton Table ============

interface SkeletonTableProps {
  rows?: number
  columns?: number
}

export function SkeletonTable({ rows = 5, columns = 4 }: SkeletonTableProps) {
  return (
    <div className="space-y-3">
      {/* Header */}
      <div className="flex gap-4 pb-2 border-b">
        {Array.from({ length: columns }).map((_, i) => (
          <Skeleton key={i} className="h-4 w-[100px]" />
        ))}
      </div>
      {/* Rows */}
      {Array.from({ length: rows }).map((_, i) => (
        <div key={i} className="flex gap-4 py-2">
          {Array.from({ length: columns }).map((_, j) => (
            <Skeleton key={j} className="h-4 w-[100px]" />
          ))}
        </div>
      ))}
    </div>
  )
}

// ============ Skeleton List ============

interface SkeletonListProps {
  items?: number
}

export function SkeletonList({ items = 5 }: SkeletonListProps) {
  return (
    <div className="space-y-2">
      {Array.from({ length: items }).map((_, i) => (
        <div key={i} className="flex items-center gap-3 p-2">
          <Skeleton className="h-10 w-10 rounded" />
          <div className="space-y-2 flex-1">
            <Skeleton className="h-4 w-3/4" />
            <Skeleton className="h-3 w-1/2" />
          </div>
        </div>
      ))}
    </div>
  )
}

// ============ Skeleton Card ============

interface SkeletonCardProps {
  count?: number
}

export function SkeletonCard({ count = 3 }: SkeletonCardProps) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      {Array.from({ length: count }).map((_, i) => (
        <div key={i} className="border rounded-lg p-4 space-y-3">
          <Skeleton className="h-12 w-12 rounded-lg" />
          <Skeleton className="h-5 w-3/4" />
          <Skeleton className="h-4 w-1/2" />
        </div>
      ))}
    </div>
  )
}

// ============ Empty State ============

type EmptyStateIcon = 'inbox' | 'folder' | 'file' | 'search' | 'alert' | 'custom'

interface EmptyStateProps {
  icon?: EmptyStateIcon
  customIcon?: React.ReactNode
  title: string
  description?: string
  action?: {
    label: string
    onClick: () => void
  }
  className?: string
}

const iconMap = {
  inbox: Inbox,
  folder: FolderOpen,
  file: FileQuestion,
  search: Search,
  alert: AlertCircle,
  custom: null,
}

export function EmptyState({ 
  icon = 'inbox', 
  customIcon, 
  title, 
  description, 
  action,
  className 
}: EmptyStateProps) {
  const IconComponent = iconMap[icon]
  
  return (
    <div className={cn('flex flex-col items-center justify-center py-12 px-4 text-center', className)}>
      <div className="mb-4">
        {customIcon || (IconComponent && (
          <div className="p-4 rounded-full bg-muted">
            <IconComponent className="h-8 w-8 text-muted-foreground" />
          </div>
        ))}
      </div>
      <h3 className="text-lg font-medium mb-1">{title}</h3>
      {description && (
        <p className="text-sm text-muted-foreground mb-4 max-w-sm">{description}</p>
      )}
      {action && (
        <Button onClick={action.onClick} variant="outline">
          {action.label}
        </Button>
      )}
    </div>
  )
}

// ============ Error State ============

interface ErrorStateProps {
  error: string | Error
  onRetry?: () => void
  className?: string
}

export function ErrorState({ error, onRetry, className }: ErrorStateProps) {
  const errorMessage = error instanceof Error ? error.message : error
  
  return (
    <div className={cn('flex flex-col items-center justify-center py-12 px-4 text-center', className)}>
      <div className="mb-4">
        <div className="p-4 rounded-full bg-destructive/10">
          <AlertCircle className="h-8 w-8 text-destructive" />
        </div>
      </div>
      <h3 className="text-lg font-medium mb-1">Something went wrong</h3>
      <p className="text-sm text-muted-foreground mb-4 max-w-sm">{errorMessage}</p>
      {onRetry && (
        <Button onClick={onRetry} variant="outline">
          Try Again
        </Button>
      )}
    </div>
  )
}
