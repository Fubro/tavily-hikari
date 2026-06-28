#[tokio::test]
async fn startup_migration_repairs_partially_migrated_effect_bucket_rows() {
    let db_path = temp_db_path("partial-effect-bucket-migration");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
        .await
        .expect("proxy created");
    let token = proxy
        .create_access_token(Some("partial-effect-bucket-migration"))
        .await
        .expect("create token");
    drop(proxy);

    let pool = connect_sqlite_test_pool(&db_str).await;
    let now = Utc::now().timestamp();

    sqlx::query(
        r#"
        INSERT INTO request_logs (
            auth_token_id,
            method,
            path,
            result_status,
            key_effect_code,
            key_effect_summary,
            binding_effect_code,
            binding_effect_summary,
            visibility,
            created_at
        ) VALUES (?, 'POST', '/api/tavily/search', 'success', ?, 'legacy bound', ?, 'already bound', 'visible', ?)
        "#,
    )
    .bind(&token.id)
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_PRESSURE_AVOIDED)
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_BOUND)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert partially migrated request log");

    sqlx::query(
        r#"
        INSERT INTO auth_token_logs (
            token_id,
            method,
            path,
            result_status,
            key_effect_code,
            key_effect_summary,
            selection_effect_code,
            selection_effect_summary,
            created_at
        ) VALUES (?, 'POST', '/api/tavily/search', 'success', ?, 'legacy rebound', ?, 'already selected', ?)
        "#,
    )
    .bind(&token.id)
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_REBOUND)
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_PRESSURE_AVOIDED)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert partially migrated token log");
    sqlx::query("DELETE FROM meta WHERE key = ?")
        .bind(META_KEY_REQUEST_LOG_EFFECT_BUCKET_MIGRATION_V1_DONE)
        .execute(&pool)
        .await
        .expect("clear migration marker to simulate interrupted migration");
    drop(pool);

    let migrated_proxy =
        TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
            .await
            .expect("proxy reopened for partial migration repair");

    let pool = connect_sqlite_test_pool(&db_str).await;
    let request_row = sqlx::query(
        r#"
        SELECT key_effect_code, binding_effect_code, selection_effect_code
        FROM request_logs
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("read repaired request log");
    assert_eq!(
        request_row
            .try_get::<String, _>("key_effect_code")
            .expect("request key effect code"),
        KEY_EFFECT_NONE
    );
    assert_eq!(
        request_row
            .try_get::<String, _>("binding_effect_code")
            .expect("request binding effect code"),
        KEY_EFFECT_HTTP_PROJECT_AFFINITY_BOUND
    );
    assert_eq!(
        request_row
            .try_get::<String, _>("selection_effect_code")
            .expect("request selection effect code"),
        KEY_EFFECT_HTTP_PROJECT_AFFINITY_PRESSURE_AVOIDED
    );

    let token_row = sqlx::query(
        r#"
        SELECT key_effect_code, binding_effect_code, selection_effect_code
        FROM auth_token_logs
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("read repaired token log");
    assert_eq!(
        token_row
            .try_get::<String, _>("key_effect_code")
            .expect("token key effect code"),
        KEY_EFFECT_NONE
    );
    assert_eq!(
        token_row
            .try_get::<String, _>("binding_effect_code")
            .expect("token binding effect code"),
        KEY_EFFECT_HTTP_PROJECT_AFFINITY_REBOUND
    );
    assert_eq!(
        token_row
            .try_get::<String, _>("selection_effect_code")
            .expect("token selection effect code"),
        KEY_EFFECT_HTTP_PROJECT_AFFINITY_PRESSURE_AVOIDED
    );

    assert_eq!(
        migrated_proxy
            .key_store
            .get_meta_i64(META_KEY_REQUEST_LOG_EFFECT_BUCKET_MIGRATION_V1_DONE)
            .await
            .expect("read migration marker"),
        Some(1)
    );

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn effect_bucket_migration_precheck_ignores_main_request_logs_leftovers() {
    let db_path = temp_db_path("effect-bucket-migration-observability-precheck");
    let db_str = db_path.to_string_lossy().to_string();
    let proxy = TavilyProxy::with_endpoint(Vec::<String>::new(), DEFAULT_UPSTREAM, &db_str)
        .await
        .expect("proxy created");
    let pool = connect_sqlite_test_pool(&db_str).await;
    let now = Utc::now().timestamp();

    sqlx::query(
        r#"
        CREATE TABLE request_logs AS
        SELECT * FROM observability.request_logs WHERE 0
        "#,
    )
    .execute(&pool)
    .await
    .expect("create misleading main request_logs leftover");

    sqlx::query(
        r#"
        INSERT INTO request_logs (
            method,
            path,
            result_status,
            key_effect_code,
            binding_effect_code,
            selection_effect_code,
            visibility,
            created_at
        ) VALUES ('POST', '/api/tavily/search', 'success', 'none', ?, ?, 'visible', ?)
        "#,
    )
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_BOUND)
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_PRESSURE_AVOIDED)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert clean leftover main request log");

    sqlx::query(
        r#"
        INSERT INTO observability.request_logs (
            method,
            path,
            result_status,
            key_effect_code,
            key_effect_summary,
            binding_effect_code,
            selection_effect_code,
            visibility,
            created_at
        ) VALUES ('POST', '/api/tavily/search', 'success', ?, 'legacy bound', 'none', 'none', 'visible', ?)
        "#,
    )
    .bind(KEY_EFFECT_HTTP_PROJECT_AFFINITY_BOUND)
    .bind(now)
    .execute(&pool)
    .await
    .expect("insert dirty observability request log");

    sqlx::query("DELETE FROM meta WHERE key = ?")
        .bind(META_KEY_REQUEST_LOG_EFFECT_BUCKET_MIGRATION_V1_DONE)
        .execute(&pool)
        .await
        .expect("clear migration marker");

    proxy
        .key_store
        .rerun_log_effect_bucket_migration_for_test()
        .await
        .expect("rerun effect-bucket migration");

    let observability_row = sqlx::query(
        r#"
        SELECT key_effect_code, binding_effect_code
        FROM observability.request_logs
        ORDER BY id DESC
        LIMIT 1
        "#,
    )
    .fetch_one(&pool)
    .await
    .expect("read repaired observability row");
    assert_eq!(
        observability_row
            .try_get::<String, _>("key_effect_code")
            .expect("observability key effect code"),
        KEY_EFFECT_NONE
    );
    assert_eq!(
        observability_row
            .try_get::<String, _>("binding_effect_code")
            .expect("observability binding effect code"),
        KEY_EFFECT_HTTP_PROJECT_AFFINITY_BOUND
    );

    assert_eq!(
        proxy
            .key_store
            .get_meta_i64(META_KEY_REQUEST_LOG_EFFECT_BUCKET_MIGRATION_V1_DONE)
            .await
            .expect("read migration marker"),
        Some(1)
    );

    let _ = std::fs::remove_file(db_path);
}
