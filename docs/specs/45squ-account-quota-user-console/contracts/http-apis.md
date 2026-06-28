# HTTP APIs

## GET /api/user/dashboard

- Scope: external
- Change: New
- Auth: `hikari_user_session` cookie

### Query

- optional: `today_start`, `today_end`
- contract:
  - must be RFC3339 / ISO8601 datetimes with explicit offset or `Z`
  - must be provided together
  - must describe exactly one natural-day window aligned to local midnight
  - omitted pair falls back to the server-timezone current day window

### Response

- `200`
- body:
  - `requestRate { used, limit, windowMinutes, scope }`
  - `businessCalls1h { totalCount, successCount, failureCount, limit, windowMinutes }`
  - `dailyCreditsUsed`, `dailyCreditsLimit`
  - `monthlyCreditsUsed`, `monthlyCreditsLimit`
  - `dailySuccess`, `dailyFailure`, `monthlySuccess`
  - `lastActivity`

### Semantics

- `dailySuccess` / `dailyFailure`: explicit browser-window today when query params are present; otherwise server timezone current day
- `monthlySuccess`: current UTC month
- `businessCalls1h`: 最近 1 小时实际上游业务请求次数；成功与失败都计入，前置拦截与 `quota_exhausted` 不计入
- `dailyCreditsUsed` / `dailyCreditsLimit`: server timezone natural day
- `monthlyCreditsUsed` / `monthlyCreditsLimit`: current UTC month

### Error

- `401` 未登录
- `404` OAuth 功能未启用

## GET /api/user/tokens

- Scope: external
- Change: New
- Auth: `hikari_user_session`

### Query

- optional: `today_start`, `today_end`
- same validation contract as `GET /api/user/dashboard`

### Response

- `200`
- body: `UserTokenSummary[]`
  - `tokenId`, `enabled`, `note`, `lastUsedAt`
  - `requestRate`
  - `businessCalls1h`
  - `dailyCreditsUsed/dailyCreditsLimit`
  - `monthlyCreditsUsed/monthlyCreditsLimit`
  - `dailySuccess`, `dailyFailure`, `monthlySuccess`

## GET /api/user/tokens/:id

- Scope: external
- Change: New
- Auth: `hikari_user_session`

### Query

- optional: `today_start`, `today_end`
- same validation contract as `GET /api/user/dashboard`

### Response

- `200` `UserTokenSummary`

### Error

- `401` 未登录
- `404` token 不属于当前用户或 OAuth 未启用

## GET /api/user/tokens/:id/secret

- Scope: external
- Change: New
- Auth: `hikari_user_session`

### Response

- `200` `{ "token": "th-<id>-<secret>" }`

### Error

- `401` 未登录
- `404` token 不属于当前用户或不可用

## GET /api/user/tokens/:id/logs?limit=20

- Scope: external
- Change: New
- Auth: `hikari_user_session`

### Response

- `200` `PublicTokenLog[]`（已做敏感字段脱敏）

### Error

- `401` 未登录
- `404` token 不属于当前用户或 OAuth 未启用

## Route changes

- `GET /auth/linuxdo` 生成登录 state 时默认 `redirect_to=/console`。
- `GET /` 当用户 session 有效时返回 `302 /console`。
- 新增 `GET /console` 与 `GET /console/` 页面入口。
