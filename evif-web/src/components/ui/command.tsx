import * as React from "react"
import { cn } from "@/lib/utils"

interface CommandProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: React.ReactNode
}

const Command = React.forwardRef<HTMLDivElement, CommandProps>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "flex h-full w-full flex-col overflow-hidden rounded-md bg-popover text-popover-foreground",
        className
      )}
      {...props}
    >
      {children}
    </div>
  )
)
Command.displayName = "Command"

interface CommandListProps extends React.HTMLAttributes<HTMLDivElement> {
  children?: React.ReactNode
}

const CommandList = React.forwardRef<HTMLDivElement, CommandListProps>(
  ({ className, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "max-h-[300px] overflow-y-auto overflow-x-hidden",
        className
      )}
      {...props}
    >
      {children}
    </div>
  )
)
CommandList.displayName = "CommandList"

interface CommandInputProps extends React.InputHTMLAttributes<HTMLInputElement> {}

const CommandInput = React.forwardRef<HTMLInputElement, CommandInputProps>(
  ({ className, ...props }, ref) => (
    <input
      ref={ref}
      className={cn(
        "flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background file:border-0 file:bg-transparent file:text-sm file:font-medium placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50",
        className
      )}
      {...props}
    />
  )
)
CommandInput.displayName = "CommandInput"

interface CommandGroupProps extends React.HTMLAttributes<HTMLDivElement> {
  heading?: string
  children?: React.ReactNode
}

const CommandGroup = React.forwardRef<HTMLDivElement, CommandGroupProps>(
  ({ className, heading, children, ...props }, ref) => (
    <div
      ref={ref}
      className={cn(
        "overflow-hidden p-1 text-foreground",
        className
      )}
      {...props}
    >
      {heading && (
        <div className="px-2 py-1.5 text-xs font-semibold text-muted-foreground">
          {heading}
        </div>
      )}
      {children}
    </div>
  )
)
CommandGroup.displayName = "CommandGroup"

interface CommandItemProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  children?: React.ReactNode
}

const CommandItem = React.forwardRef<HTMLButtonElement, CommandItemProps>(
  ({ className, children, ...props }, ref) => (
    <button
      ref={ref}
      className={cn(
        "relative flex cursor-default select-none items-center rounded-sm px-2 py-1.5 text-sm outline-none hover:bg-accent hover:text-accent-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
        className
      )}
      {...props}
    >
      {children}
    </button>
  )
)
CommandItem.displayName = "CommandItem"

interface CommandSeparatorProps extends React.HTMLAttributes<HTMLDivElement> {}

const CommandSeparator = React.forwardRef<HTMLDivElement, CommandSeparatorProps>(
  ({ className, ...props }, ref) => (
    <div
      ref={ref}
      className={cn("-mx-1 h-px bg-border", className)}
      {...props}
    />
  )
)
CommandSeparator.displayName = "CommandSeparator"

export {
  Command,
  CommandList,
  CommandInput,
  CommandGroup,
  CommandItem,
  CommandSeparator,
}
