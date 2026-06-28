use super::*;

#[tokio::test]
async fn startup_migrates_legacy_quota_schema_to_semantic_columns() {
    let db_path = temp_db_path("legacy-quota-schema-migration");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
        .await
        .expect("proxy created");
    let user = proxy
        .upsert_oauth_account(&OAuthAccountProfile {
            provider: "linuxdo".to_string(),
            provider_user_id: "legacy-quota-schema-user".to_string(),
            username: Some("legacy_quota_schema_user".to_string()),
            name: Some("Legacy Quota Schema User".to_string()),
            avatar_template: None,
            active: true,
            trust_level: Some(2),
            raw_payload_json: None,
        })
        .await
        .expect("upsert user");
    drop(proxy);

    let pool = connect_sqlite_test_pool(&db_str).await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("disable fk checks");

    sqlx::query("DROP TABLE IF EXISTS account_quota_limit_snapshots")
        .execute(&pool)
        .await
        .expect("drop semantic snapshot table");
    sqlx::query("DROP TABLE IF EXISTS account_quota_limits")
        .execute(&pool)
        .await
        .expect("drop semantic quota limits table");
    sqlx::query("DROP TABLE IF EXISTS user_tag_bindings")
        .execute(&pool)
        .await
        .expect("drop semantic user tag bindings table");
    sqlx::query("DROP TABLE IF EXISTS user_tags")
        .execute(&pool)
        .await
        .expect("drop semantic user tags table");

    sqlx::query(
        r#"
        CREATE TABLE account_quota_limits (
            user_id TEXT PRIMARY KEY,
            hourly_any_limit INTEGER NOT NULL,
            hourly_limit INTEGER NOT NULL,
            daily_limit INTEGER NOT NULL,
            monthly_limit INTEGER NOT NULL,
            monthly_broken_limit INTEGER NOT NULL DEFAULT 5,
            monthly_blocked_key_limit_delta INTEGER NOT NULL DEFAULT 0,
            inherits_defaults INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create legacy quota limits table");

    sqlx::query(
        r#"
        CREATE TABLE account_quota_limit_snapshots (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id TEXT NOT NULL,
            changed_at INTEGER NOT NULL,
            hourly_any_limit INTEGER NOT NULL,
            hourly_limit INTEGER NOT NULL,
            daily_limit INTEGER NOT NULL,
            monthly_limit INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create legacy quota snapshot table");

    sqlx::query(
        r#"
        CREATE TABLE user_tags (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            icon TEXT,
            system_key TEXT UNIQUE,
            effect_kind TEXT NOT NULL DEFAULT 'quota_delta',
            hourly_any_delta INTEGER NOT NULL DEFAULT 0,
            hourly_delta INTEGER NOT NULL DEFAULT 0,
            daily_delta INTEGER NOT NULL DEFAULT 0,
            monthly_delta INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create legacy user tags table");

    let changed_at = Utc::now().timestamp() - 120;
    sqlx::query(
        r#"
        INSERT INTO account_quota_limits (
            user_id,
            hourly_any_limit,
            hourly_limit,
            daily_limit,
            monthly_limit,
            monthly_broken_limit,
            monthly_blocked_key_limit_delta,
            inherits_defaults,
            created_at,
            updated_at
        ) VALUES (?, 60, 12, 345, 6789, 7, 2, 0, ?, ?)
        "#,
    )
    .bind(&user.user_id)
    .bind(changed_at)
    .bind(changed_at)
    .execute(&pool)
    .await
    .expect("insert legacy quota limits row");

    sqlx::query(
        r#"
        INSERT INTO account_quota_limit_snapshots (
            user_id,
            changed_at,
            hourly_any_limit,
            hourly_limit,
            daily_limit,
            monthly_limit
        ) VALUES (?, ?, 60, 12, 345, 6789)
        "#,
    )
    .bind(&user.user_id)
    .bind(changed_at)
    .execute(&pool)
    .await
    .expect("insert legacy quota snapshot row");

    sqlx::query(
        r#"
        INSERT INTO user_tags (
            id,
            name,
            display_name,
            icon,
            system_key,
            effect_kind,
            hourly_any_delta,
            hourly_delta,
            daily_delta,
            monthly_delta,
            created_at,
            updated_at
        ) VALUES (?, 'legacy_vip', 'Legacy VIP', 'star', NULL, 'quota_delta', 999, 4, 5, 6, ?, ?)
        "#,
    )
    .bind("tag_legacy_vip")
    .bind(changed_at)
    .bind(changed_at)
    .execute(&pool)
    .await
    .expect("insert legacy tag row");

    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("re-enable fk checks");
    drop(pool);

    let proxy_after = TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
        .await
        .expect("proxy reopened with legacy schema");

    let pragma_rows: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('account_quota_limits')")
            .fetch_all(&proxy_after.key_store.pool)
            .await
            .expect("quota limits pragma");
    assert!(pragma_rows.contains(&"business_calls_1h_limit".to_string()));
    assert!(pragma_rows.contains(&"daily_credits_limit".to_string()));
    assert!(pragma_rows.contains(&"monthly_credits_limit".to_string()));
    assert!(!pragma_rows.contains(&"hourly_any_limit".to_string()));
    assert!(!pragma_rows.contains(&"hourly_limit".to_string()));
    assert!(!pragma_rows.contains(&"daily_limit".to_string()));
    assert!(!pragma_rows.contains(&"monthly_limit".to_string()));

    let snapshot_pragma_rows: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('account_quota_limit_snapshots')")
            .fetch_all(&proxy_after.key_store.pool)
            .await
            .expect("snapshot pragma");
    assert!(snapshot_pragma_rows.contains(&"business_calls_1h_limit".to_string()));
    assert!(snapshot_pragma_rows.contains(&"daily_credits_limit".to_string()));
    assert!(snapshot_pragma_rows.contains(&"monthly_credits_limit".to_string()));
    assert!(!snapshot_pragma_rows.contains(&"hourly_any_limit".to_string()));

    let tag_pragma_rows: Vec<String> =
        sqlx::query_scalar("SELECT name FROM pragma_table_info('user_tags')")
            .fetch_all(&proxy_after.key_store.pool)
            .await
            .expect("tag pragma");
    assert!(tag_pragma_rows.contains(&"business_calls_1h_delta".to_string()));
    assert!(tag_pragma_rows.contains(&"daily_credits_delta".to_string()));
    assert!(tag_pragma_rows.contains(&"monthly_credits_delta".to_string()));
    assert!(!tag_pragma_rows.contains(&"hourly_any_delta".to_string()));
    assert!(!tag_pragma_rows.contains(&"hourly_delta".to_string()));
    assert!(!tag_pragma_rows.contains(&"daily_delta".to_string()));
    assert!(!tag_pragma_rows.contains(&"monthly_delta".to_string()));

    let migrated_limits: (i64, i64, i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            business_calls_1h_limit,
            daily_credits_limit,
            monthly_credits_limit,
            monthly_broken_limit,
            monthly_blocked_key_limit_delta,
            inherits_defaults
        FROM account_quota_limits
        WHERE user_id = ?
        "#,
    )
    .bind(&user.user_id)
    .fetch_one(&proxy_after.key_store.pool)
    .await
    .expect("read migrated quota limits");
    assert_eq!(migrated_limits, (12, 345, 6789, 7, 2, 0));

    let migrated_snapshot: (i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT business_calls_1h_limit, daily_credits_limit, monthly_credits_limit
        FROM account_quota_limit_snapshots
        WHERE user_id = ?
        AND changed_at = ?
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .bind(&user.user_id)
    .bind(changed_at)
    .fetch_one(&proxy_after.key_store.pool)
    .await
    .expect("read migrated quota snapshot");
    assert_eq!(migrated_snapshot, (12, 345, 6789));

    let migrated_tag = proxy_after
        .list_user_tags()
        .await
        .expect("list migrated user tags")
        .into_iter()
        .find(|tag| tag.id == "tag_legacy_vip")
        .expect("legacy tag preserved");
    assert_eq!(migrated_tag.business_calls_1h_delta, 4);
    assert_eq!(migrated_tag.daily_credits_delta, 5);
    assert_eq!(migrated_tag.monthly_credits_delta, 6);
    proxy_after
        .bind_user_tag_to_user(&user.user_id, &migrated_tag.id)
        .await
        .expect("bind migrated tag");
    let quota_details = proxy_after
        .get_admin_user_quota_details(&user.user_id)
        .await
        .expect("read admin quota details")
        .expect("quota details present");
    assert_eq!(quota_details.base.business_calls_1h_limit, 12);
    assert_eq!(quota_details.base.daily_credits_limit, 345);
    assert_eq!(quota_details.base.monthly_credits_limit, 6789);
    assert_eq!(quota_details.effective.business_calls_1h_limit, 116);
    assert_eq!(quota_details.effective.daily_credits_limit, 850);
    assert_eq!(quota_details.effective.monthly_credits_limit, 11795);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn startup_quota_schema_migration_is_idempotent() {
    let db_path = temp_db_path("legacy-quota-schema-idempotent");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
        .await
        .expect("proxy created");
    let user = proxy
        .upsert_oauth_account(&OAuthAccountProfile {
            provider: "linuxdo".to_string(),
            provider_user_id: "legacy-quota-schema-idempotent-user".to_string(),
            username: Some("legacy_quota_schema_idempotent_user".to_string()),
            name: Some("Legacy Quota Schema Idempotent User".to_string()),
            avatar_template: None,
            active: true,
            trust_level: Some(1),
            raw_payload_json: None,
        })
        .await
        .expect("upsert user");
    drop(proxy);

    let pool = connect_sqlite_test_pool(&db_str).await;
    sqlx::query("PRAGMA foreign_keys = OFF")
        .execute(&pool)
        .await
        .expect("disable fk checks");
    sqlx::query("DROP TABLE IF EXISTS account_quota_limit_snapshots")
        .execute(&pool)
        .await
        .expect("drop semantic snapshot table");
    sqlx::query("DROP TABLE IF EXISTS account_quota_limits")
        .execute(&pool)
        .await
        .expect("drop semantic quota limits table");
    sqlx::query("DROP TABLE IF EXISTS user_tag_bindings")
        .execute(&pool)
        .await
        .expect("drop semantic user tag bindings table");
    sqlx::query("DROP TABLE IF EXISTS user_tags")
        .execute(&pool)
        .await
        .expect("drop semantic user tags table");
    sqlx::query(
        r#"
        CREATE TABLE account_quota_limits (
            user_id TEXT PRIMARY KEY,
            hourly_any_limit INTEGER NOT NULL,
            hourly_limit INTEGER NOT NULL,
            daily_limit INTEGER NOT NULL,
            monthly_limit INTEGER NOT NULL,
            monthly_broken_limit INTEGER NOT NULL DEFAULT 5,
            monthly_blocked_key_limit_delta INTEGER NOT NULL DEFAULT 0,
            inherits_defaults INTEGER NOT NULL DEFAULT 1,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id)
        )
        "#,
    )
    .execute(&pool)
    .await
    .expect("create legacy quota table");
    sqlx::query(
        r#"
        INSERT INTO account_quota_limits (
            user_id,
            hourly_any_limit,
            hourly_limit,
            daily_limit,
            monthly_limit,
            monthly_broken_limit,
            monthly_blocked_key_limit_delta,
            inherits_defaults,
            created_at,
            updated_at
        ) VALUES (?, 60, 22, 333, 4444, 5, 0, 1, 1, 1)
        "#,
    )
    .bind(&user.user_id)
    .execute(&pool)
    .await
    .expect("insert legacy quota row");
    sqlx::query("PRAGMA foreign_keys = ON")
        .execute(&pool)
        .await
        .expect("re-enable fk checks");
    drop(pool);

    let proxy_after_first =
        TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
            .await
            .expect("first reopen");
    let first_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM account_quota_limits WHERE user_id = ?")
            .bind(&user.user_id)
            .fetch_one(&proxy_after_first.key_store.pool)
            .await
            .expect("count quota rows after first migration");
    assert_eq!(first_rows, 1);
    drop(proxy_after_first);

    let proxy_after_second =
        TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
            .await
            .expect("second reopen");
    let second_limits: (i64, i64, i64, i64) = sqlx::query_as(
        r#"
        SELECT
            business_calls_1h_limit,
            daily_credits_limit,
            monthly_credits_limit,
            inherits_defaults
        FROM account_quota_limits
        WHERE user_id = ?
        "#,
    )
    .bind(&user.user_id)
    .fetch_one(&proxy_after_second.key_store.pool)
    .await
    .expect("read migrated limits after second reopen");
    assert_eq!(second_limits, (100, 500, 5000, 1));
    let second_rows: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM account_quota_limits WHERE user_id = ?")
            .bind(&user.user_id)
            .fetch_one(&proxy_after_second.key_store.pool)
            .await
            .expect("count quota rows after second migration");
    assert_eq!(second_rows, 1);

    let _ = std::fs::remove_file(db_path);
}
