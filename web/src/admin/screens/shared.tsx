import type { ReactNode } from 'react'

import { ArrowDown, ArrowUp, ArrowUpDown } from 'lucide-react'

import AdminCompactIntro from '../../components/AdminCompactIntro'
import { Badge } from '../../components/ui/badge'
import { Button } from '../../components/ui/button'
import { Tooltip, TooltipContent, TooltipTrigger } from '../../components/ui/tooltip'
import type { AdminUserTagBinding, SortDirection } from '../../api'
import type { AdminTranslations } from '../../i18n'

export type StackedValue = {
  primary: string
  secondary?: string | null
  primaryClassName?: string | null
}

export function AdminTableValueStack({
  primary,
  secondary,
  primaryClassName,
}: StackedValue): JSX.Element {
  return (
    <div className="admin-table-value-stack">
      <span className={`admin-table-value-primary${primaryClassName ? ` ${primaryClassName}` : ''}`}>
        {primary}
      </span>
      {secondary ? <span className="admin-table-value-secondary">{secondary}</span> : null}
    </div>
  )
}

export function MonthlyBrokenCountTrigger({
  count,
  onOpen,
  ariaLabel,
  className,
}: {
  count: number
  onOpen?: (() => void) | null
  ariaLabel: string
  className?: string | null
}): JSX.Element {
  const primary = String(Math.max(0, count))
  if (count <= 0 || !onOpen) {
    return <span className={`admin-table-value-primary${className ? ` ${className}` : ''}`}>{primary}</span>
  }
  return (
    <button
      type="button"
      className={`link-button admin-table-value-link${className ? ` ${className}` : ''}`}
      onClick={onOpen}
      aria-label={ariaLabel}
    >
      {primary}
    </button>
  )
}

export function AdminUsersSortableHeader<Field extends string>({
  label,
  displayLabel,
  tooltipLabel,
  field,
  activeField,
  activeOrder,
  onToggle,
}: {
  label: string
  displayLabel?: string
  tooltipLabel?: string
  field: Field
  activeField: Field | null
  activeOrder: SortDirection | null
  onToggle: (field: Field) => void
}): JSX.Element {
  const isActive = activeField === field
  const ariaSort = !isActive ? 'none' : activeOrder === 'asc' ? 'ascending' : 'descending'
  const SortIndicatorIcon = !isActive ? ArrowUpDown : activeOrder === 'asc' ? ArrowUp : ArrowDown
  const visibleLabel = displayLabel ?? label
  const bubbleLabel = tooltipLabel ?? label
  const hasTooltip = bubbleLabel.trim() !== visibleLabel.trim()
  const trigger = (
    <Button
      type="button"
      variant="ghost"
      size="sm"
      data-sort-field={field}
      className={`admin-table-sort-button${isActive ? ' is-active' : ''}`}
      onClick={() => onToggle(field)}
      aria-label={hasTooltip ? bubbleLabel : undefined}
    >
      <span className="admin-table-sort-label">{visibleLabel}</span>
      <SortIndicatorIcon className="admin-table-sort-indicator" aria-hidden="true" />
    </Button>
  )
  return (
    <th aria-sort={ariaSort}>
      {hasTooltip ? (
        <Tooltip>
          <TooltipTrigger asChild>{trigger}</TooltipTrigger>
          <TooltipContent className="max-w-[min(18rem,calc(100vw-2rem))]" side="top">
            {bubbleLabel}
          </TooltipContent>
        </Tooltip>
      ) : (
        trigger
      )}
    </th>
  )
}

export function UsagePageIntro({
  title,
  description,
  searchControls,
  filterStatusText,
  filterStatusTestId = 'users-filter-status',
}: {
  title: string
  description: string
  searchControls: ReactNode
  filterStatusText?: string | null
  filterStatusTestId?: string
}): JSX.Element {
  return (
    <>
      <div className="admin-desktop-only">
        <AdminCompactIntro title={title} description={description} actions={searchControls} />
      </div>
      <div className="admin-stacked-only">
        <section className="surface app-header admin-usage-stacked-intro">
          <div className="admin-usage-stacked-intro-main">
            <h1>{title}</h1>
            <p className="admin-compact-intro-description">{description}</p>
          </div>
          <div className="admin-usage-stacked-intro-actions">{searchControls}</div>
        </section>
      </div>
      {filterStatusText ? (
        <p className="panel-description admin-usage-filter-status" data-testid={filterStatusTestId}>
          {filterStatusText}
        </p>
      ) : null}
    </>
  )
}

type UserTagLike = Pick<AdminUserTagBinding, 'displayName' | 'icon' | 'systemKey' | 'effectKind'> & {
  source?: string | null
}

export function getUserTagIconSrc(icon: string | null | undefined): string | null {
  if (icon === 'linuxdo') {
    return '/assets/linuxdo-logo.svg'
  }
  return null
}

export function isSystemUserTag(tag: { systemKey?: string | null; source?: string | null }): boolean {
  return Boolean(tag.systemKey) || tag.source === 'system_linuxdo'
}

export function UserTagBadge({
  tag,
  usersStrings,
}: {
  tag: UserTagLike
  usersStrings: AdminTranslations['users']
}): JSX.Element {
  const iconSrc = getUserTagIconSrc(tag.icon)
  const isSystem = isSystemUserTag(tag)
  const isBlockAll = tag.effectKind === 'block_all'
  const classes = [
    'user-tag-pill',
    isSystem ? 'user-tag-pill-system' : '',
    isBlockAll ? 'user-tag-pill-block' : '',
  ]
    .filter(Boolean)
    .join(' ')

  return (
    <Badge variant="outline" className={classes} title={tag.displayName}>
      {iconSrc ? <img src={iconSrc} alt="" className="user-tag-pill-icon" aria-hidden="true" /> : null}
      <span>{tag.displayName}</span>
      {isSystem ? <span className="user-tag-pill-meta">{usersStrings.catalog.scopeSystemShort}</span> : null}
      {isBlockAll ? <span className="user-tag-pill-meta">{usersStrings.catalog.blockShort}</span> : null}
    </Badge>
  )
}

export function UserTagBadgeList({
  tags,
  usersStrings,
  emptyLabel,
  limit = 3,
}: {
  tags: AdminUserTagBinding[]
  usersStrings: AdminTranslations['users']
  emptyLabel: string
  limit?: number
}): JSX.Element {
  if (tags.length === 0) {
    return <span className="panel-description">{emptyLabel}</span>
  }

  const visibleTags = tags.slice(0, limit)
  const overflow = Math.max(0, tags.length - visibleTags.length)

  return (
    <div className="user-tag-pill-list">
      {visibleTags.map((tag) => (
        <UserTagBadge key={`${tag.tagId}:${tag.source}`} tag={tag} usersStrings={usersStrings} />
      ))}
      {overflow > 0 ? <Badge variant="outline" className="user-tag-pill-overflow">+{overflow}</Badge> : null}
    </div>
  )
}

export function formatUnboundTokenIdentityMeta(
  note: string | null,
  group: string | null,
  groupLabel: string,
): string {
  const parts: string[] = []
  const normalizedNote = note?.trim() ?? ''
  const normalizedGroup = group?.trim() ?? ''
  if (normalizedNote) parts.push(normalizedNote)
  if (normalizedGroup) parts.push(`${groupLabel} ${normalizedGroup}`)
  return parts.join(' · ') || '—'
}

export function formatSuccessRateStackValue(
  success: number,
  failure: number,
  language: 'en' | 'zh',
): StackedValue {
  const total = success + failure
  const percent = total === 0 ? '—' : `${(Math.round((success / total) * 1000) / 10).toFixed(1)}%`
  return {
    primary: percent,
    secondary:
      language === 'zh'
        ? `成 ${success} / 败 ${failure}`
        : `S ${success} / F ${failure}`,
  }
}

export function formatCompactSuccessRateValue(
  success: number,
  failure: number,
  language: 'en' | 'zh',
): string {
  const total = success + failure
  const percent = total === 0 ? '—' : `${(Math.round((success / total) * 1000) / 10).toFixed(1)}%`
  return language === 'zh'
    ? `${percent} 成 ${success} / 败 ${failure}`
    : `${percent} S ${success} / F ${failure}`
}
