use super::*;

#[tokio::test]
async fn public_success_breakdown_skips_flush_when_no_pending_request_stats() {
    let db_path = temp_db_path("public-success-breakdown-no-pending-flush");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-public-success-no-pending".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");

    proxy
        .key_store
        .set_meta_i64(
            META_KEY_REQUEST_STATS_LAST_FLUSHED_AT_V1,
            Utc::now().timestamp(),
        )
        .await
        .expect("set request stats flush watermark");

    let now = Utc::now().timestamp();
    let window = TimeRangeUtc {
        start: now.saturating_sub(300),
        end: now.saturating_add(60),
    };
    let public = proxy
        .success_breakdown(Some(window))
        .await
        .expect("public success breakdown");

    assert_eq!(public.monthly_success, 0);
    assert_eq!(public.daily_success, 0);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn public_success_breakdown_flushes_pending_request_stats_for_current_window() {
    let db_path = temp_db_path("public-success-breakdown-pending-flush");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-public-success-pending".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");

    let key_id = proxy
        .list_api_key_metrics()
        .await
        .expect("list key metrics")
        .into_iter()
        .next()
        .expect("seeded key")
        .id;
    let now = Utc::now().timestamp();
    let window = TimeRangeUtc {
        start: now.saturating_sub(300),
        end: now.saturating_add(60),
    };

    proxy
        .key_store
        .enqueue_request_stats_rollup_for_test(
            Some(&key_id),
            now.saturating_sub(10),
            OUTCOME_SUCCESS,
        )
        .await;

    let public = proxy
        .success_breakdown(Some(window))
        .await
        .expect("public success breakdown");

    assert_eq!(public.monthly_success, 1);
    assert_eq!(public.daily_success, 1);

    let persisted_flush = proxy
        .key_store
        .get_meta_i64(META_KEY_REQUEST_STATS_LAST_FLUSHED_AT_V1)
        .await
        .expect("read request stats flush watermark");
    assert!(persisted_flush.unwrap_or_default() > 0);

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn public_success_breakdown_flushes_when_newer_pending_rollup_is_inside_window() {
    let db_path = temp_db_path("public-success-breakdown-mixed-pending-window");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-public-success-mixed-pending".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");

    let key_id = proxy
        .list_api_key_metrics()
        .await
        .expect("list key metrics")
        .into_iter()
        .next()
        .expect("seeded key")
        .id;
    let now = Utc::now().timestamp();
    let day_start = now.saturating_sub(300);
    let window = TimeRangeUtc {
        start: day_start,
        end: now.saturating_add(60),
    };

    proxy
        .key_store
        .set_meta_i64(
            META_KEY_REQUEST_STATS_LAST_FLUSHED_AT_V1,
            day_start.saturating_sub(5),
        )
        .await
        .expect("set request stats flush watermark");

    proxy
        .key_store
        .enqueue_request_stats_rollup_for_test(
            Some(&key_id),
            day_start.saturating_sub(120),
            OUTCOME_SUCCESS,
        )
        .await;
    proxy
        .key_store
        .enqueue_request_stats_rollup_for_test(
            Some(&key_id),
            now.saturating_sub(10),
            OUTCOME_SUCCESS,
        )
        .await;

    let public = proxy
        .success_breakdown(Some(window))
        .await
        .expect("public success breakdown");

    assert_eq!(public.monthly_success, 2);
    assert_eq!(public.daily_success, 1);

    let persisted_flush = proxy
        .key_store
        .get_meta_i64(META_KEY_REQUEST_STATS_LAST_FLUSHED_AT_V1)
        .await
        .expect("read request stats flush watermark");
    assert!(persisted_flush.unwrap_or_default() >= now.saturating_sub(10));

    let _ = std::fs::remove_file(db_path);
}

#[tokio::test]
async fn public_success_breakdown_flushes_when_pending_rollup_is_outside_day_but_inside_month() {
    let db_path = temp_db_path("public-success-breakdown-month-only-pending");
    let db_str = db_path.to_string_lossy().to_string();

    let proxy = TavilyProxy::with_endpoint(
        vec!["tvly-public-success-month-only-pending".to_string()],
        DEFAULT_UPSTREAM,
        &db_str,
    )
    .await
    .expect("proxy created");

    let key_id = proxy
        .list_api_key_metrics()
        .await
        .expect("list key metrics")
        .into_iter()
        .next()
        .expect("seeded key")
        .id;
    let now = Utc::now();
    let month_start = start_of_month(now).timestamp();
    let day_window = server_local_day_window_utc(now.with_timezone(&Local));
    let pending_created_at = day_window.start.saturating_sub(120);
    assert!(pending_created_at >= month_start);

    proxy
        .key_store
        .enqueue_request_stats_rollup_for_test(Some(&key_id), pending_created_at, OUTCOME_SUCCESS)
        .await;

    let public = proxy
        .success_breakdown(Some(day_window))
        .await
        .expect("public success breakdown");

    assert_eq!(public.monthly_success, 1);
    assert_eq!(public.daily_success, 0);

    let _ = std::fs::remove_file(db_path);
}
