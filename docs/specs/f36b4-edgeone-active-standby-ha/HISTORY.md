# History

## Decision

The project originally considered a master/slave split with multiple serving slaves and centralized token quota dispatch. The accepted direction is single-active active/standby because EdgeOne free tier is suitable as a single-domain origin switching control plane, not as an application load balancer.

## Rationale

Single-active reduces quota, rebalance, conversation remapping, and upstream key ownership conflicts. Existing MCP Rebalance and API Rebalance remain single-active instance capabilities. Automatic failover intentionally stops at `provisional_master` so core API/MCP traffic recovers quickly while high-risk writes require an administrator decision.

## Accepted Semantics

- EdgeOne current origin is the active-master authority, including while nodes are already running.
- A node that was active and later observes EdgeOne pointing elsewhere must enter `recovery` and stop external business service.
- A standby that observes EdgeOne pointing at itself is not silently trusted as `full_master`; it becomes `provisional_master` until an administrator finalizes.
- Recovery import is mergeable-only. It must not import request or auth-token log rows, and it must not overwrite settings, current quota/token/key state, or rebalance authority state.
- Non-force promote is a standby operation. Active-node promote attempts are rejected so operator error cannot produce a local double-active state.

## Small-State Sync Revision

Production validation showed that full SQLite snapshot sync is unsafe for the current data shape because request logs and response bodies can make the database tens of GiB. The accepted replacement is standby-pulled, zstd-compressed NDJSON state sync over explicit whitelisted resources plus a 72-hour `ha_outbox` event stream. HA sync is now forbidden from transporting full database files or raw request/auth-token log records.

## Multi-Channel HA Revision

Production `ha_outbox` growth showed that the old single whitelist simultaneously controlled
baseline export, event emission, and trigger install, so hot tables such as `billing_ledger` and
runtime quota state could silently flood the same `ha_outbox`. The accepted replacement splits HA
into explicit `control`, `billing`, and `runtime` channels with independent baselines, event
streams, and peer watermarks.

- `control` stays a small control-plane event stream in `ha_outbox`.
- `billing` gets dedicated `billing_ledger` replication through `ha_billing_outbox`.
- `runtime` carries only minimal API/MCP correctness state through `ha_runtime_outbox`.
- Mixed-version HA is no longer supported. A future standby must cold start from the same build and
  then join using the new channel-aware contract.

## HA Outbox Maintenance Revision

Large historical `ha_outbox` cleanup is now an explicit maintenance action, not a startup side
effect. The accepted sequence is:

1. stop new HA writes or keep the node in `HA_MODE=single`
2. run the offline `ha_outbox_cleanup_once` / `scripts/ha-outbox-maintenance.sh`
3. run `db_compaction_once` only if reclaimable space justifies it

The scheduler still performs bounded online `ha_outbox_gc`, but that path is intentionally limited
to per-channel expired-row cleanup plus a passive WAL checkpoint so it cannot recreate the previous
SWAP spike failure mode.

## Full Live Snapshot Validation Revision

Merge-ready validation for the HA outbox fix now requires a complete production-shaped SQLite input
copied from 101 into `codex-testbox`. “Complete” is explicitly the core DB plus the observability
sidecar sibling, not the main `.db` file alone.

- The accepted source path on 101 remains the Docker volume backing `/srv/app/data`.
- The accepted transfer shape is a read-only SQLite `.backup` snapshot set, not sampled tables and
  not a hand-picked subset of rows.
- Offline HA cleanup and optional compaction proof now runs against that copied snapshot set inside
  one isolated testbox run directory.

## Admin IA Revision

The full HA node inventory is an operations setting, not global business-page chrome. Admin business pages stay silent in normal `full_master` state and show only a compact attention link during abnormal HA states; promote/finalize remains confined to the System Settings high-availability subpage.

## Source Settings Revision

The admin HA page now treats the current instance source as a private, per-node setting that can be switched between direct `IP/域名` and `源站组`. The stored value overrides the Env/CLI default for this instance only, and active/provisional operators can apply the saved source directly to EdgeOne from the same page. Startup defaults now accept `HA_SOURCE_KIND` and `HA_SOURCE_ORIGIN_GROUP_ID`, while `EDGEONE_EXPECTED_ORIGIN_*` remains a direct-origin compatibility input.

## Source Settings Contract Hardening

HA source settings originally drifted across layers: the frontend, demo fixtures, and spec already used lowercase `http|https|follow`, but the Rust enum deserializer still expected PascalCase variants, causing direct-origin saves to fail before the handler executed. The accepted contract is now explicitly lowercase on the HA admin JSON wire, with the uppercase `HTTP|HTTPS|FOLLOW` mapping confined to the downstream EdgeOne control-plane payload.

## Source Settings Failure UX Hardening

The HA source settings dialog previously dumped raw backend failure text straight into the modal body, which was both visually inconsistent and hard to scan. The accepted interaction keeps local input validation beside the affected field and reserves form-level remote failures for a formal destructive alert with operator-friendly copy plus a default-collapsed technical-details disclosure.
