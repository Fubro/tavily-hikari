import { cn } from '../lib/utils'

interface BrandWordmarkProps {
  title?: string
  compact?: boolean
  className?: string
  markClassName?: string
}

export default function BrandLockup({
  title = 'Tavily Hikari',
  compact = false,
  className,
  markClassName,
}: BrandWordmarkProps): JSX.Element {
  return (
    <span className={cn('brand-lockup', compact && 'brand-lockup-compact', className)}>
      <img
        src="/relay-mesh-lockup.png"
        alt={title}
        className={cn('brand-lockup-image', compact && 'brand-lockup-image-compact', markClassName)}
        loading="eager"
        decoding="async"
      />
    </span>
  )
}
