import { type ClassValue, clsx } from "clsx"
import { twMerge } from "tailwind-merge"

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs))
}

/**
 * Check if running on macOS
 */
export const isMac = typeof navigator !== 'undefined' && navigator.platform.includes('Mac')
