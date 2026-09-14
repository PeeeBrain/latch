import { useEffect, useRef, useState, type ReactNode } from 'react'
import { LucideIcon, Eye, EyeOff, Copy } from 'lucide-react'
import { useClipboardGuard } from '../hooks/useClipboardGuard'

interface PaletteInputProps {
  value: string
  onChange: (value: string) => void
  onSubmit?: () => void
  placeholder: string
  type?: 'text' | 'password'
  icon?: LucideIcon
  autoFocus?: boolean
  hint?: string
  disabled?: boolean
  iconSpin?: boolean
  action?: ReactNode
}

function PaletteInput({
  value,
  onChange,
  onSubmit,
  placeholder,
  type = 'text',
  icon: Icon,
  autoFocus = true,
  hint,
  disabled = false,
  iconSpin = false,
  action,
}: PaletteInputProps) {
  const inputRef = useRef<HTMLInputElement>(null)
  const [showPassword, setShowPassword] = useState(false)
  const { copy } = useClipboardGuard()

  useEffect(() => {
    if (autoFocus && inputRef.current) {
      inputRef.current.focus()
    }
  }, [autoFocus])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter' && onSubmit) {
      e.preventDefault()
      onSubmit()
    }
  }

  const isPassword = type === 'password'
  const inputType = isPassword ? (showPassword ? 'text' : 'password') : type

  return (
    <div className="border-b-2 border-theme-accent bg-theme-accent">
      <div className="flex items-center px-5 py-4 gap-4">
        {Icon && (
          <Icon
            className={`text-2xl flex-shrink-0 text-theme-text ${iconSpin ? 'animate-spin' : ''}`}
            size={20}
          />
        )}
        <input
          ref={inputRef}
          type={inputType}
          value={value}
          onChange={(e) => onChange(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          disabled={disabled}
          className={`flex-1 bg-transparent border-none outline-none text-[20px] leading-[1.2] ${isPassword ? 'font-password' : 'font-theme'} text-theme-text placeholder:text-theme-text-inverse/60 disabled:opacity-50 disabled:cursor-not-allowed`}
        />
        {isPassword && value && (
          <div className="flex gap-2">
            <button
              type="button"
              onClick={() => setShowPassword(!showPassword)}
              title={showPassword ? 'Hide password' : 'Show password'}
              className="flex items-center justify-center w-11 h-11 bg-theme-bg border-2 border-theme-accent text-theme-text cursor-pointer transition-transform duration-100 hover:bg-theme-accent hover:text-theme-accent-text hover:translate-x-[1px] hover:translate-y-[1px] shadow-theme-sm"
            >
              {showPassword ? <EyeOff size={16} /> : <Eye size={16} />}
            </button>
            <button
              type="button"
              onClick={() => copy(value)}
              title="Copy password"
              className="flex items-center justify-center w-11 h-11 bg-theme-bg border-2 border-theme-accent text-theme-text cursor-pointer transition-transform duration-100 hover:bg-theme-accent hover:text-theme-accent-text hover:translate-x-[1px] hover:translate-y-[1px] shadow-theme-sm"
            >
              <Copy size={16} />
            </button>
          </div>
        )}
        {action}
        {hint && (
          <span className="text-xs text-theme-bg bg-theme-text border-2 border-theme-text font-extrabold flex-shrink-0 tracking-wider uppercase shadow-theme-sm px-3 py-1.5">
            {hint}
          </span>
        )}
      </div>
    </div>
  )
}

export default PaletteInput



