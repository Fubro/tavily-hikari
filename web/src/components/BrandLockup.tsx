import { useId } from 'react'

import { cn } from '../lib/utils'

interface BrandMarkProps {
  className?: string
  accentClassName?: string
  strokeClassName?: string
  haloClassName?: string
}

export function BrandMark({
  className,
  accentClassName = 'text-[hsl(var(--secondary))]',
  strokeClassName = 'text-[hsl(var(--foreground))]',
  haloClassName = 'text-[hsl(var(--primary)/0.16)]',
}: BrandMarkProps): JSX.Element {
  const meshId = useId().replace(/:/g, '')
  const strokeGradientId = `relay-mesh-stroke-${meshId}`
  const accentGradientId = `relay-mesh-accent-${meshId}`

  return (
    <span className={cn('brand-mark', className)} aria-hidden="true">
      <svg viewBox="0 0 80 80" className="brand-mark-svg" role="presentation">
        <defs>
          <linearGradient id={strokeGradientId} x1="14" y1="10" x2="64" y2="68" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="currentColor" />
            <stop offset="100%" stopColor="hsl(var(--primary))" />
          </linearGradient>
          <linearGradient id={accentGradientId} x1="44" y1="18" x2="67" y2="42" gradientUnits="userSpaceOnUse">
            <stop offset="0%" stopColor="hsl(var(--secondary))" />
            <stop offset="100%" stopColor="hsl(var(--primary))" />
          </linearGradient>
        </defs>
        <circle cx="40" cy="40" r="31" className={cn('brand-mark-halo', haloClassName)} />
        <path
          d="M18 44C18 29.64 29.64 18 44 18H55.5L62 24.5V36"
          fill="none"
          stroke={`url(#${strokeGradientId})`}
          strokeWidth="7"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={cn('brand-mark-stroke', strokeClassName)}
        />
        <path
          d="M62 36C62 50.36 50.36 62 36 62H24.5L18 55.5V44"
          fill="none"
          stroke={`url(#${strokeGradientId})`}
          strokeWidth="7"
          strokeLinecap="round"
          strokeLinejoin="round"
          className={cn('brand-mark-stroke', strokeClassName)}
        />
        <path
          d="M31.5 29.5L48.5 29.5L48.5 35.5L41.5 35.5L41.5 50.5L35.5 50.5L35.5 35.5L31.5 35.5Z"
          className={cn('brand-mark-accent', accentClassName)}
          fill={`url(#${accentGradientId})`}
        />
        <circle cx="58.5" cy="22.5" r="5.5" className={cn('brand-mark-accent', accentClassName)} fill={`url(#${accentGradientId})`} />
        <circle cx="21.5" cy="57.5" r="5.5" className={cn('brand-mark-accent', accentClassName)} fill={`url(#${accentGradientId})`} />
      </svg>
    </span>
  )
}

interface BrandWordmarkProps {
  title?: string
  subtitle?: string
  compact?: boolean
  className?: string
  titleClassName?: string
  subtitleClassName?: string
  markClassName?: string
  markAccentClassName?: string
  markStrokeClassName?: string
  markHaloClassName?: string
}

export default function BrandLockup({
  title = 'Tavily Hikari',
  subtitle,
  compact = false,
  className,
  titleClassName,
  subtitleClassName,
  markClassName,
  markAccentClassName,
  markStrokeClassName,
  markHaloClassName,
}: BrandWordmarkProps): JSX.Element {
  return (
    <span className={cn('brand-lockup', compact && 'brand-lockup-compact', className)}>
      <BrandMark
        className={cn('brand-lockup-mark', compact && 'brand-lockup-mark-compact', markClassName)}
        accentClassName={markAccentClassName}
        strokeClassName={markStrokeClassName}
        haloClassName={markHaloClassName}
      />
      <span className="brand-lockup-copy">
        <span className={cn('brand-lockup-title', titleClassName)}>{title}</span>
        {subtitle ? <span className={cn('brand-lockup-subtitle', subtitleClassName)}>{subtitle}</span> : null}
      </span>
    </span>
  )
}
