import * as React from 'react'
import { cn } from '@/lib/utils'

interface SwitchProps {
  id?: string
  checked?: boolean
  disabled?: boolean
  className?: string
  onCheckedChange?: (checked: boolean) => void
}

export const Switch = React.forwardRef<HTMLButtonElement, SwitchProps>(
  ({ id, checked = false, disabled = false, className, onCheckedChange }, ref) => {
    const handleClick = () => {
      if (disabled) return
      onCheckedChange?.(!checked)
    }

    return (
      <button
        id={id}
        type="button"
        aria-pressed={checked}
        aria-disabled={disabled}
        onClick={handleClick}
        ref={ref}
        className={cn(
          'inline-flex items-center h-6 w-11 rounded-full transition-colors',
          'border-2 border-transparent',
          disabled ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer',
          checked ? 'bg-primary' : 'bg-muted',
          className
        )}
      >
        <span
          className={cn(
            'block h-5 w-5 rounded-full bg-foreground shadow-lg transition-transform',
            checked ? 'translate-x-[18px] bg-background' : 'translate-x-0'
          )}
        />
      </button>
    )
  }
)
Switch.displayName = 'Switch'
