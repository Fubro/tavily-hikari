import { Icon } from '../lib/icons'
import type { HaStatus } from '../api'

interface HaStatusBannerProps {
  status: HaStatus | null
  audience: 'admin' | 'user'
  onPromote?: () => void
  onFinalize?: () => void
  busy?: boolean
}

function roleLabel(role: HaStatus['role']): string {
  if (role === 'full_master') return 'Full master'
  if (role === 'provisional_master') return 'Provisional master'
  if (role === 'standby') return 'Standby'
  return 'Recovery'
}

function formatTimestamp(value: number | null): string {
  if (value == null) return 'unknown'
  return new Date(value * 1000).toLocaleString()
}

export default function HaStatusBanner({
  status,
  audience,
  onPromote,
  onFinalize,
  busy = false,
}: HaStatusBannerProps): JSX.Element | null {
  const admin = audience === 'admin'
  if (!status || status.mode === 'single' || (!admin && !status.degraded)) return null

  const title = status.role === 'provisional_master'
    ? 'Failover is active but not finalized'
    : status.role === 'standby'
      ? 'This node is in standby'
      : status.role === 'recovery'
        ? 'This node is in recovery'
        : 'This node is the active master'
  const detail = status.role === 'provisional_master'
    ? 'API and MCP traffic can continue. Registration, recharge, and configuration writes stay disabled until an administrator finalizes failover.'
    : status.role === 'standby'
      ? 'This node is syncing and should not handle external writes. Promote only when the current EdgeOne origin is unhealthy.'
      : status.role === 'recovery'
        ? 'Only mergeable usage, log, event, and payment notification data should be imported from this node.'
        : 'Full business writes are enabled on this node. Standby nodes should continue receiving snapshots.'
  const toneClass = status.role === 'full_master' ? 'ha-status-banner-active' : ''

  return (
    <section className={`ha-status-banner ${toneClass}`} role="status" aria-live="polite">
      <div className="ha-status-banner-icon" aria-hidden="true">
        <Icon icon={status.role === 'full_master' ? 'mdi:check-circle-outline' : 'mdi:alert-circle-outline'} width={22} height={22} />
      </div>
      <div className="ha-status-banner-copy">
        <div className="ha-status-banner-title">{title}</div>
        <p>{detail}</p>
        {admin && (
          <dl className="ha-status-banner-meta">
            <div><dt>Node</dt><dd>{status.nodeId}</dd></div>
            <div><dt>Role</dt><dd>{roleLabel(status.role)}</dd></div>
            <div><dt>Node origin</dt><dd>{status.nodePublicOrigin ?? 'unknown'}</dd></div>
            <div><dt>EdgeOne domain</dt><dd>{status.edgeoneDomain ?? 'unknown'}</dd></div>
            <div><dt>EdgeOne origin</dt><dd>{status.edgeoneOrigin ?? 'unknown'}</dd></div>
            <div><dt>Expected origin</dt><dd>{status.edgeoneExpectedOrigin ?? 'unknown'}</dd></div>
            <div><dt>EdgeOne API</dt><dd>{status.edgeoneApiConfigured ? 'configured' : 'not configured'}</dd></div>
            <div><dt>Sync lag</dt><dd>{status.syncLagSeconds == null ? 'unknown' : `${status.syncLagSeconds}s`}</dd></div>
            <div><dt>Last EdgeOne check</dt><dd>{formatTimestamp(status.lastEdgeoneCheckAt)}</dd></div>
            <div><dt>Last sync</dt><dd>{formatTimestamp(status.lastSyncAt)}</dd></div>
            <div><dt>Basic traffic</dt><dd>{status.allowsBasicBusiness ? 'allowed' : 'blocked'}</dd></div>
            <div><dt>Full writes</dt><dd>{status.allowsFullWrites ? 'allowed' : 'blocked'}</dd></div>
            <div><dt>Recovery</dt><dd>{status.recoveryStatus ?? 'none'}</dd></div>
            <div><dt>Message</dt><dd>{status.message ?? 'none'}</dd></div>
          </dl>
        )}
      </div>
      {admin && (
        <div className="ha-status-banner-actions">
          {status.role === 'standby' && onPromote && (
            <button type="button" onClick={onPromote} disabled={busy}>
              Promote
            </button>
          )}
          {status.role === 'provisional_master' && onFinalize && (
            <button type="button" onClick={onFinalize} disabled={busy}>
              Finalize
            </button>
          )}
        </div>
      )}
    </section>
  )
}
