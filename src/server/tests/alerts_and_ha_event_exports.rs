use super::*;
use super::core_support_and_parsing::*;
use super::upstream_support_and_manual_jobs::*;

#[tokio::test]
async fn ha_events_endpoint_returns_zstd_ndjson() {
    let db_path = temp_db_path("ha-events");
    let db_str = db_path.to_string_lossy().to_string();
    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-ha-events-key".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");
    let pool = connect_sqlite_test_pool(&db_str).await;
    sqlx::query(
        r#"
        INSERT INTO ha_outbox (
            kind, resource, resource_id, op, payload_json, created_at, checksum
        ) VALUES ('state', 'api_keys', 'key-1', 'upsert', '{"id":"key-1"}', ?, 'checksum')
        "#,
    )
    .bind(Utc::now().timestamp())
    .execute(&pool)
    .await
    .expect("insert outbox event");
    sqlx::query(
        r#"
        INSERT INTO ha_outbox (
            kind, resource, resource_id, op, payload_json, created_at, checksum
        ) VALUES ('state', 'request_logs', 'log-1', 'upsert', '{"path":"/secret"}', ?, 'bad')
        "#,
    )
    .bind(Utc::now().timestamp())
    .execute(&pool)
    .await
    .expect("insert forbidden outbox event");
    pool.close().await;
    let ha = tavily_hikari::HaRuntime::new(tavily_hikari::HaConfig {
        node_id: "node-events".to_string(),
        database_path: Some(db_str.clone()),
        ..tavily_hikari::HaConfig::default()
    });
    let addr = spawn_ha_admin_server(proxy, ha, true).await;
    let response = Client::new()
        .get(format!(
            "http://{addr}/api/admin/ha/events?channel=control&after=0&limit=10"
        ))
        .send()
        .await
        .expect("events request");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("content-encoding")
            .and_then(|value| value.to_str().ok()),
        Some("zstd")
    );
    assert_eq!(
        response
            .headers()
            .get("x-ha-event-count")
            .and_then(|value| value.to_str().ok()),
        Some("1")
    );
    assert_eq!(
        response
            .headers()
            .get("x-ha-last-seq")
            .and_then(|value| value.to_str().ok()),
        Some("1")
    );
    let compressed = response.bytes().await.expect("events bytes");
    let decoded = zstd::stream::decode_all(compressed.as_ref()).expect("decode zstd events");
    let text = String::from_utf8(decoded).expect("events utf8");
    assert!(text.contains("\"kind\":\"event\""));
    assert!(text.contains("\"resource\":\"api_keys\""));
    assert!(!text.contains("request_logs"));
    assert!(!text.contains("/secret"));
    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn ha_events_endpoint_chunks_oversized_batches() {
    let db_path = temp_db_path("ha-events-chunked-batches");
    let db_str = db_path.to_string_lossy().to_string();
    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-ha-events-chunked-batches".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");
    let pool = connect_sqlite_test_pool(&db_str).await;
    let now = Utc::now().timestamp();
    sqlx::query(
        r#"
        INSERT INTO ha_outbox (
            kind, resource, resource_id, op, payload_json, created_at, checksum
        ) VALUES ('state', 'meta', ?, 'upsert', ?, ?, NULL)
        "#,
    )
    .bind("request_rate_limit_v1-1")
    .bind(
        serde_json::json!({
            "key": "request_rate_limit_v1",
            "value": format!("1-{}", nanoid!(512))
        })
        .to_string(),
    )
    .bind(now + 1)
    .execute(&pool)
    .await
    .expect("insert first outbox event");
    let ha = tavily_hikari::HaRuntime::new(tavily_hikari::HaConfig {
        node_id: "node-events-chunked".to_string(),
        database_path: Some(db_str.clone()),
        ..tavily_hikari::HaConfig::default()
    });
    let addr = spawn_ha_admin_server(proxy, ha, true).await;
    let client = Client::new();

    let single = client
        .get(format!(
            "http://{addr}/api/admin/ha/events?channel=control&after=0&limit=4"
        ))
        .send()
        .await
        .expect("single events request");
    assert_eq!(single.status(), reqwest::StatusCode::OK);
    let single_compressed = single.bytes().await.expect("single events bytes");

    for seq in 2..=4 {
        sqlx::query(
            r#"
            INSERT INTO ha_outbox (
                kind, resource, resource_id, op, payload_json, created_at, checksum
            ) VALUES ('state', 'meta', ?, 'upsert', ?, ?, NULL)
            "#,
        )
        .bind(format!("request_rate_limit_v1-{seq}"))
        .bind(
            serde_json::json!({
                "key": "request_rate_limit_v1",
                "value": format!("{}-{}", seq, nanoid!(512))
            })
            .to_string(),
        )
        .bind(now + i64::from(seq))
        .execute(&pool)
        .await
        .expect("insert outbox event");
    }
    pool.close().await;

    let _cap = EnvVarGuard::set(
        "TAVILY_TEST_HA_EVENTS_MAX_COMPRESSED_BYTES",
        &single_compressed.len().to_string(),
    );
    let response = client
        .get(format!(
            "http://{addr}/api/admin/ha/events?channel=control&after=0&limit=4"
        ))
        .send()
        .await
        .expect("chunked events request");
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get("x-ha-event-count")
            .and_then(|value| value.to_str().ok()),
        Some("1")
    );
    assert_eq!(
        response
            .headers()
            .get("x-ha-last-seq")
            .and_then(|value| value.to_str().ok()),
        Some("1")
    );
    let compressed = response.bytes().await.expect("chunked events bytes");
    assert!(
        compressed.len() <= single_compressed.len(),
        "chunked response should honor compressed cap"
    );
    let decoded = zstd::stream::decode_all(compressed.as_ref()).expect("decode chunked events");
    let text = String::from_utf8(decoded).expect("chunked events utf8");
    assert_eq!(text.matches("\"kind\":\"event\"").count(), 1);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn ha_events_endpoint_returns_413_when_single_event_exceeds_cap() {
    let _cap = EnvVarGuard::set("TAVILY_TEST_HA_EVENTS_MAX_COMPRESSED_BYTES", "128");
    let db_path = temp_db_path("ha-events-single-over-cap");
    let db_str = db_path.to_string_lossy().to_string();
    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-ha-events-single-over-cap".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");
    let pool = connect_sqlite_test_pool(&db_str).await;
    sqlx::query(
        r#"
        INSERT INTO ha_outbox (
            kind, resource, resource_id, op, payload_json, created_at, checksum
        ) VALUES ('state', 'meta', 'request_rate_limit_v1', 'upsert', ?, ?, NULL)
        "#,
    )
    .bind(
        serde_json::json!({
            "key": "request_rate_limit_v1",
            "value": nanoid!(4096)
        })
        .to_string(),
    )
    .bind(Utc::now().timestamp())
    .execute(&pool)
    .await
    .expect("insert oversized outbox event");
    pool.close().await;
    let ha = tavily_hikari::HaRuntime::new(tavily_hikari::HaConfig {
        node_id: "node-events-over-cap".to_string(),
        database_path: Some(db_str.clone()),
        ..tavily_hikari::HaConfig::default()
    });
    let addr = spawn_ha_admin_server(proxy, ha, true).await;
    let response = Client::new()
        .get(format!(
            "http://{addr}/api/admin/ha/events?channel=control&after=0&limit=1"
        ))
        .send()
        .await
        .expect("oversized events request");
    assert_eq!(response.status(), reqwest::StatusCode::PAYLOAD_TOO_LARGE);
    let body = response.text().await.expect("oversized events body");
    assert!(
        body.contains("HA events payload exceeds compressed limit"),
        "unexpected events error body: {body}"
    );

    let _ = std::fs::remove_file(db_path);
}
