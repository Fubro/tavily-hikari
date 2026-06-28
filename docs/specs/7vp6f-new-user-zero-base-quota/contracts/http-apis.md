## GET `/api/users/:id`

- `quotaBase` 对“新创建且尚无标签、且此前没有额度行的用户”返回：
  - `businessCalls1hLimit = 0`
  - `dailyCreditsLimit = 0`
  - `monthlyCreditsLimit = 0`
  - `inheritsDefaults = false`
- `effectiveQuota` 继续返回“基线 + 标签增量”的结果。

## GET `/api/user/tokens` / GET `/api/user/tokens/:id`

- 对已绑定账户 token：
  - `businessCalls1h.limit`、`dailyCreditsLimit`、`monthlyCreditsLimit` 继续从账户有效额度派生。
  - 若用户无标签且基线为 0，则 limit 也为 0。
- 对未绑定 token：
  - 非账户绑定 token 继续沿用现有 token 默认额度，不受本轮账户零基线影响。

## PATCH `/api/users/:id/quota`

- payload 改为：
  - `businessCalls1hLimit`
  - `dailyCreditsLimit`
  - `monthlyCreditsLimit`
- 写入账户基线后，后续读取 `quotaBase` / `effectiveQuota` 按现有规则反映管理员设置值。
