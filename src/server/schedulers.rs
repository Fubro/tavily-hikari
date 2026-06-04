fn random_delay_secs(max_inclusive: u64) -> u64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    rng.gen_range(0..=max_inclusive)
}

fn twenty_four_hours_secs() -> i64 {
    24 * 60 * 60
}

fn two_hours_secs() -> i64 {
    2 * 60 * 60
}

fn fifteen_minutes_secs() -> i64 {
    15 * 60
}

fn forward_proxy_geo_refresh_recheck_secs() -> i64 {
    60
}

fn request_logs_gc_catchup_recheck_secs() -> u64 {
    900
}

fn scheduled_request_logs_gc_options() -> RequestLogsGcOptions {
    RequestLogsGcOptions {
        batch_size: 25,
        max_batches: 20,
        max_runtime_secs: 60,
        inter_batch_sleep_ms: 2_000,
    }
}

fn db_compaction_min_reclaimable_bytes() -> u64 {
    512 * 1024 * 1024
}

fn db_compaction_min_reclaimable_ratio() -> f64 {
    0.20
}

fn db_compaction_cooldown_secs() -> u64 {
    24 * 60 * 60
}

const LINUXDO_USER_STATUS_SYNC_JOB_TYPE: &str = "linuxdo_user_status_sync";
const LINUXDO_USER_TAG_BINDING_REFRESH_JOB_TYPE: &str = "linuxdo_user_tag_binding_refresh";
const TRIGGER_SOURCE_SCHEDULER: &str = "scheduler";
const TRIGGER_SOURCE_MANUAL: &str = "manual";
const TRIGGER_SOURCE_AUTO: &str = "auto";

async fn claim_scheduled_job(
    state: &AppState,
    job_type: &str,
    key_id: Option<&str>,
    trigger_source: &str,
    log_prefix: &str,
) -> Option<i64> {
    match state
        .proxy
        .scheduled_job_claim(job_type, trigger_source, key_id, 1)
        .await
    {
        Ok(Some(id)) => Some(id),
        Ok(None) => {
            eprintln!("{log_prefix}: job already running; skip trigger");
            None
        }
        Err(err) => {
            eprintln!("{log_prefix}: start job error: {err}");
            None
        }
    }
}

fn next_local_daily_run_after(now: DateTime<Local>, hour: u32, minute: u32) -> DateTime<Local> {
    let today = now.date_naive();
    let scheduled_naive = today
        .and_hms_opt(hour, minute, 0)
        .unwrap_or_else(|| today.and_hms_opt(6, 20, 0).expect("valid default time"));
    let scheduled_today = match Local.from_local_datetime(&scheduled_naive) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => now,
    };
    if scheduled_today > now {
        return scheduled_today;
    }

    let tomorrow = today.succ_opt().unwrap_or_else(|| {
        today
            .checked_add_days(chrono::Days::new(1))
            .unwrap_or(today)
    });
    let next_naive = tomorrow
        .and_hms_opt(hour, minute, 0)
        .unwrap_or_else(|| tomorrow.and_hms_opt(6, 20, 0).expect("valid default time"));
    match Local.from_local_datetime(&next_naive) {
        chrono::LocalResult::Single(dt) => dt,
        chrono::LocalResult::Ambiguous(dt, _) => dt,
        chrono::LocalResult::None => now + ChronoDuration::hours(24),
    }
}

fn duration_until_next_local_daily_run(now: DateTime<Local>, hour: u32, minute: u32) -> Duration {
    (next_local_daily_run_after(now, hour, minute) - now)
        .to_std()
        .unwrap_or_else(|_| Duration::from_secs(0))
}

fn spawn_quota_sync_scheduler(state: Arc<AppState>) {
    let cold_state = state.clone();
    tokio::spawn(async move {
        loop {
            let keys = match cold_state
                .proxy
                .list_keys_pending_quota_sync(twenty_four_hours_secs())
                .await
            {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("quota-sync: list pending error: {err}");
                    vec![]
                }
            };

            for key_id in keys {
                let delay = random_delay_secs(300);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                let Some(job_id) = claim_scheduled_job(
                    cold_state.as_ref(),
                    "quota_sync",
                    Some(&key_id),
                    TRIGGER_SOURCE_SCHEDULER,
                    "quota-sync",
                )
                .await
                else {
                    continue;
                };
                match cold_state
                    .proxy
                    .sync_key_quota(&key_id, &cold_state.usage_base, "quota_sync")
                    .await
                {
                    Ok((limit, remaining)) => {
                        let msg = format!("limit={limit} remaining={remaining}");
                        let _ = cold_state
                            .proxy
                            .scheduled_job_finish(job_id, "success", Some(&msg))
                            .await;
                    }
                    Err(ProxyError::QuotaDataMissing { reason }) => {
                        let msg = format!("quota_data_missing: {reason}");
                        let _ = cold_state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&msg))
                            .await;
                    }
                    Err(ProxyError::UsageHttp { status, body }) => {
                        let msg = format!("usage_http {status}: {body}");
                        let _ = cold_state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&msg))
                            .await;
                    }
                    Err(err) => {
                        let _ = cold_state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                            .await;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });

    let hot_state = state;
    tokio::spawn(async move {
        loop {
            let keys = match hot_state
                .proxy
                .list_keys_pending_hot_quota_sync(two_hours_secs(), fifteen_minutes_secs())
                .await
            {
                Ok(list) => list,
                Err(err) => {
                    eprintln!("quota-sync-hot: list pending error: {err}");
                    vec![]
                }
            };

            for key_id in keys {
                let delay = random_delay_secs(60);
                tokio::time::sleep(Duration::from_secs(delay)).await;
                let Some(job_id) = claim_scheduled_job(
                    hot_state.as_ref(),
                    "quota_sync/hot",
                    Some(&key_id),
                    TRIGGER_SOURCE_SCHEDULER,
                    "quota-sync-hot",
                )
                .await
                else {
                    continue;
                };
                match hot_state
                    .proxy
                    .sync_key_quota(&key_id, &hot_state.usage_base, "quota_sync/hot")
                    .await
                {
                    Ok((limit, remaining)) => {
                        let msg = format!("limit={limit} remaining={remaining}");
                        let _ = hot_state
                            .proxy
                            .scheduled_job_finish(job_id, "success", Some(&msg))
                            .await;
                    }
                    Err(ProxyError::QuotaDataMissing { reason }) => {
                        let msg = format!("quota_data_missing: {reason}");
                        let _ = hot_state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&msg))
                            .await;
                    }
                    Err(ProxyError::UsageHttp { status, body }) => {
                        let msg = format!("usage_http {status}: {body}");
                        let _ = hot_state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&msg))
                            .await;
                    }
                    Err(err) => {
                        let _ = hot_state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                            .await;
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });
}

fn spawn_token_usage_rollup_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let Some(job_id) = claim_scheduled_job(
                state.as_ref(),
                "token_usage_rollup",
                None,
                TRIGGER_SOURCE_SCHEDULER,
                "token-usage-rollup",
            )
            .await
            else {
                tokio::time::sleep(Duration::from_secs(300)).await;
                continue;
            };

            match state.proxy.rollup_token_usage_stats().await {
                Ok((rows, last_ts)) => {
                    let msg = match last_ts {
                        Some(ts) => format!("rows={rows} last_rollup_ts={ts}"),
                        None => format!("rows={rows} last_rollup_ts=none"),
                    };
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "success", Some(&msg))
                        .await;
                }
                Err(err) => {
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                        .await;
                }
            }

            // Run rollup every 5 minutes to keep charts reasonably fresh
            tokio::time::sleep(Duration::from_secs(300)).await;
        }
    });
}

fn spawn_auth_token_logs_gc_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let Some(job_id) = claim_scheduled_job(
                state.as_ref(),
                "auth_token_logs_gc",
                None,
                TRIGGER_SOURCE_SCHEDULER,
                "auth-token-logs-gc",
            )
            .await
            else {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                continue;
            };

            match state.proxy.gc_auth_token_logs().await {
                Ok(deleted) => {
                    let msg = format!("deleted_rows={deleted}");
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "success", Some(&msg))
                        .await;
                }
                Err(err) => {
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                        .await;
                }
            }

            // Run GC once per hour; retention window is enforced inside the proxy.
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

fn spawn_mcp_sessions_gc_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let Some(job_id) = claim_scheduled_job(
                state.as_ref(),
                "mcp_sessions_gc",
                None,
                TRIGGER_SOURCE_SCHEDULER,
                "mcp-sessions-gc",
            )
            .await
            else {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                continue;
            };

            match state.proxy.gc_mcp_sessions().await {
                Ok(deleted) => {
                    let msg = format!("deleted_rows={deleted}");
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "success", Some(&msg))
                        .await;
                }
                Err(err) => {
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                        .await;
                }
            }

            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

fn spawn_mcp_session_init_backoffs_gc_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let Some(job_id) = claim_scheduled_job(
                state.as_ref(),
                "mcp_session_init_backoffs_gc",
                None,
                TRIGGER_SOURCE_SCHEDULER,
                "mcp-session-init-backoffs-gc",
            )
            .await
            else {
                tokio::time::sleep(Duration::from_secs(3600)).await;
                continue;
            };

            match state.proxy.gc_mcp_session_init_backoffs().await {
                Ok(deleted) => {
                    let msg = format!("deleted_rows={deleted}");
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "success", Some(&msg))
                        .await;
                }
                Err(err) => {
                    let _ = state
                        .proxy
                        .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                        .await;
                }
            }

            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}

fn spawn_request_logs_gc_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        // Schedule: daily at configured local time.
        loop {
            let (hour, minute) = effective_request_logs_gc_at();
            tokio::time::sleep(duration_until_next_local_daily_run(Local::now(), hour, minute))
                .await;

            // After we reach the scheduled time, keep retrying until we either run the job
            // successfully or record an error for this run window.
            loop {
                let Some(job_id) = claim_scheduled_job(
                    state.as_ref(),
                    "request_logs_gc",
                    None,
                    TRIGGER_SOURCE_SCHEDULER,
                    "request-logs-gc",
                )
                .await
                else {
                    tokio::time::sleep(Duration::from_secs(300)).await;
                    continue;
                };

                match state
                    .proxy
                    .gc_request_logs_with_options(scheduled_request_logs_gc_options())
                    .await
                {
                    Ok(report) => {
                        let msg = format!(
                            "cleaned_bodies={} deleted_rows={} rollup_deleted={} completed={} retention_days={} batches={} elapsed_ms={}",
                            report.cleaned_request_log_bodies,
                            report.deleted_request_logs,
                            report.deleted_rollups,
                            report.completed,
                            report.retention_days,
                            report.batches,
                            report.elapsed_ms
                        );
                        let _ = state
                            .proxy
                            .scheduled_job_finish(job_id, "success", Some(&msg))
                            .await;
                        if report.completed {
                            break;
                        }
                        tokio::time::sleep(Duration::from_secs(
                            request_logs_gc_catchup_recheck_secs(),
                        ))
                        .await;
                    }
                    Err(err) => {
                        let _ = state
                            .proxy
                            .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                            .await;
                        break;
                    }
                }
            }
        }
    });
}

async fn record_linuxdo_user_sync_failure(
    state: &AppState,
    provider_user_id: &str,
    attempted_at: i64,
    error: &str,
) {
    if let Err(mark_err) = state
        .proxy
        .record_oauth_account_profile_sync_failure(
            "linuxdo",
            provider_user_id,
            attempted_at,
            error,
        )
        .await
    {
        eprintln!(
            "linuxdo-user-sync: record failure metadata error for {}: {}",
            provider_user_id, mark_err
        );
    }
}

async fn run_linuxdo_user_status_sync_job(state: Arc<AppState>) {
    run_linuxdo_user_status_sync_job_with_source(state, TRIGGER_SOURCE_SCHEDULER).await;
}

async fn run_linuxdo_user_status_sync_job_with_source(
    state: Arc<AppState>,
    trigger_source: &'static str,
) {
    let Some(job_id) = claim_scheduled_job(
        state.as_ref(),
        LINUXDO_USER_STATUS_SYNC_JOB_TYPE,
        None,
        trigger_source,
        "linuxdo-user-sync",
    )
    .await
    else {
        return;
    };

    run_linuxdo_user_status_sync_claimed_job(state, job_id).await;
}

async fn run_linuxdo_user_status_sync_claimed_job(state: Arc<AppState>, job_id: i64) {
    let cfg = &state.linuxdo_oauth;
    if !cfg.is_enabled_and_configured() {
        let _ = state
            .proxy
            .scheduled_job_finish(
                job_id,
                "success",
                Some("attempted=0 success=0 skipped=0 failure=0 reason=linuxdo_oauth_not_configured"),
            )
            .await;
        return;
    }
    if !cfg.has_refresh_token_crypt_key() {
        let _ = state
            .proxy
            .scheduled_job_finish(
                job_id,
                "success",
                Some("attempted=0 success=0 skipped=0 failure=0 reason=missing_refresh_token_crypt_key"),
            )
            .await;
        return;
    }

    let records = match state.proxy.list_oauth_accounts_with_refresh_token("linuxdo").await {
        Ok(records) => records,
        Err(err) => {
            let _ = state
                .proxy
                .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                .await;
            return;
        }
    };

    if records.is_empty() {
        let _ = state
            .proxy
            .scheduled_job_finish(
                job_id,
                "success",
                Some("attempted=0 success=0 skipped=0 failure=0 reason=no_eligible_accounts"),
            )
            .await;
        return;
    }

    let client = reqwest::Client::new();
    let attempted = records.len();
    let mut success = 0usize;
    let skipped = 0usize;
    let mut failure = 0usize;
    let mut first_failure: Option<String> = None;

    for record in records {
        let attempted_at = Utc::now().timestamp();
        let record_label = record
            .username
            .as_deref()
            .or(record.name.as_deref())
            .unwrap_or(record.provider_user_id.as_str())
            .to_string();
        let refresh_token = match decrypt_linuxdo_refresh_token(
            cfg,
            &record.refresh_token_ciphertext,
            &record.refresh_token_nonce,
        ) {
            Ok(refresh_token) => refresh_token,
            Err(err) => {
                let message = err.to_string();
                failure += 1;
                first_failure
                    .get_or_insert_with(|| format!("{record_label}: {message}"));
                record_linuxdo_user_sync_failure(
                    state.as_ref(),
                    &record.provider_user_id,
                    attempted_at,
                    &message,
                )
                .await;
                continue;
            }
        };
        let (profile, token_payload) =
            match fetch_linuxdo_profile_from_refresh_token(&client, cfg, &refresh_token).await {
                Ok(result) => result,
                Err(err) => {
                    let message = err.to_string();
                    failure += 1;
                    first_failure
                        .get_or_insert_with(|| format!("{record_label}: {message}"));
                    record_linuxdo_user_sync_failure(
                        state.as_ref(),
                        &record.provider_user_id,
                        attempted_at,
                        &message,
                    )
                    .await;
                    continue;
                }
            };

        if profile.provider_user_id != record.provider_user_id {
            let message = LinuxDoSyncError::ProviderUserIdMismatch {
                expected: record.provider_user_id.clone(),
                actual: profile.provider_user_id.clone(),
            }
            .to_string();
            failure += 1;
            first_failure.get_or_insert_with(|| format!("{record_label}: {message}"));
            record_linuxdo_user_sync_failure(
                state.as_ref(),
                &record.provider_user_id,
                attempted_at,
                &message,
            )
            .await;
            continue;
        }

        let upsert_result = if let Some(rotated_refresh_token) = token_payload
            .refresh_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            match encrypt_linuxdo_refresh_token(cfg, rotated_refresh_token) {
                Ok(Some((refresh_token_ciphertext, refresh_token_nonce))) => {
                    state
                        .proxy
                        .refresh_oauth_account_profile_with_refresh_token(
                            &profile,
                            &refresh_token_ciphertext,
                            &refresh_token_nonce,
                        )
                        .await
                }
                Ok(None) => state.proxy.refresh_oauth_account_profile(&profile).await,
                Err(err) => {
                    let message = format!("encrypt rotated refresh token error: {err}");
                    failure += 1;
                    first_failure.get_or_insert_with(|| format!("{record_label}: {message}"));
                    record_linuxdo_user_sync_failure(
                        state.as_ref(),
                        &record.provider_user_id,
                        attempted_at,
                        &message,
                    )
                    .await;
                    continue;
                }
            }
        } else {
            state.proxy.refresh_oauth_account_profile(&profile).await
        };

        if let Err(err) = upsert_result {
            let mut message = format!("upsert oauth account error: {err}");
            if !profile.active
                && let Err(deactivate_err) = state
                    .proxy
                    .set_user_active_status(&record.user_id, false)
                    .await
            {
                message.push_str(&format!(
                    "; deactivate local user error: {deactivate_err}"
                ));
            }
            failure += 1;
            first_failure.get_or_insert_with(|| format!("{record_label}: {message}"));
            record_linuxdo_user_sync_failure(
                state.as_ref(),
                &record.provider_user_id,
                attempted_at,
                &message,
            )
            .await;
            continue;
        }

        if let Err(err) = state
            .proxy
            .record_oauth_account_profile_sync_success(
                "linuxdo",
                &record.provider_user_id,
                attempted_at,
            )
            .await
        {
            eprintln!(
                "linuxdo-user-sync: record success metadata error for {} (user_id={}): {}",
                record.provider_user_id, record.user_id, err
            );
        }

        success += 1;
    }

    let mut message =
        format!("attempted={attempted} success={success} skipped={skipped} failure={failure}");
    if let Some(first_failure) = first_failure {
        message.push_str(&format!(" first_failure={first_failure}"));
    }
    let final_status = if failure > 0 { "error" } else { "success" };
    let _ = state
        .proxy
        .scheduled_job_finish(job_id, final_status, Some(&message))
        .await;
}

fn spawn_linuxdo_user_status_sync_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let (hour, minute) = state.linuxdo_oauth.user_sync_time();
            tokio::time::sleep(duration_until_next_local_daily_run(Local::now(), hour, minute))
                .await;
            run_linuxdo_user_status_sync_job(state.clone()).await;
        }
    });
}

async fn run_linuxdo_user_tag_binding_refresh_job(state: Arc<AppState>) {
    run_linuxdo_user_tag_binding_refresh_job_with_source(state, TRIGGER_SOURCE_SCHEDULER).await;
}

async fn run_linuxdo_user_tag_binding_refresh_job_with_source(
    state: Arc<AppState>,
    trigger_source: &'static str,
) {
    let Some(job_id) = claim_scheduled_job(
        state.as_ref(),
        LINUXDO_USER_TAG_BINDING_REFRESH_JOB_TYPE,
        None,
        trigger_source,
        "linuxdo-tag-binding-refresh",
    )
    .await
    else {
        return;
    };

    match state.proxy.refresh_linuxdo_user_tag_bindings().await {
        Ok(refreshed) => {
            let msg = format!("refreshed={refreshed}");
            let _ = state
                .proxy
                .scheduled_job_finish(job_id, "success", Some(&msg))
                .await;
        }
        Err(err) => {
            let _ = state
                .proxy
                .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                .await;
        }
    }
}

fn spawn_linuxdo_user_tag_binding_refresh_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let wait_secs = state
                .proxy
                .linuxdo_user_tag_binding_refresh_wait_secs(twenty_four_hours_secs())
                .await;
            if wait_secs <= 0 {
                if state
                    .proxy
                    .linuxdo_user_tag_binding_refresh_due(twenty_four_hours_secs())
                    .await
                {
                    run_linuxdo_user_tag_binding_refresh_job(state.clone()).await;
                }
                tokio::time::sleep(Duration::from_secs(fifteen_minutes_secs() as u64)).await;
                continue;
            }

            let sleep_secs = wait_secs.min(fifteen_minutes_secs()) as u64;
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
        }
    });
}

async fn run_forward_proxy_geo_refresh_job(state: Arc<AppState>) {
    run_forward_proxy_geo_refresh_job_with_source(state, TRIGGER_SOURCE_SCHEDULER).await;
}

async fn run_forward_proxy_geo_refresh_job_with_source(
    state: Arc<AppState>,
    trigger_source: &'static str,
) {
    let Some(job_id) = claim_scheduled_job(
        state.as_ref(),
        "forward_proxy_geo_refresh",
        None,
        trigger_source,
        "forward-proxy-geo-refresh",
    )
    .await
    else {
        return;
    };

    match state
        .proxy
        .refresh_forward_proxy_geo_metadata(&state.api_key_ip_geo_origin, true)
        .await
    {
        Ok(refreshed) => {
            let msg = format!("refreshed_candidates={refreshed}");
            let _ = state
                .proxy
                .scheduled_job_finish(job_id, "success", Some(&msg))
                .await;
        }
        Err(err) => {
            let _ = state
                .proxy
                .scheduled_job_finish(job_id, "error", Some(&err.to_string()))
                .await;
        }
    }
}

async fn run_manual_claimed_job(
    state: Arc<AppState>,
    job_type: String,
    key_id: Option<String>,
    job_id: i64,
) {
    let finish = |state: Arc<AppState>, status: &'static str, message: String| async move {
        let _ = state
            .proxy
            .scheduled_job_finish(job_id, status, Some(&message))
            .await;
    };

    match job_type.as_str() {
        "quota_sync" => {
            let Some(key_id) = key_id else {
                finish(state, "error", "missing key_id".to_string()).await;
                return;
            };
            match state
                .proxy
                .sync_key_quota(&key_id, &state.usage_base, "quota_sync/manual")
                .await
            {
                Ok((limit, remaining)) => {
                    finish(state, "success", format!("limit={limit} remaining={remaining}")).await
                }
                Err(ProxyError::QuotaDataMissing { reason }) => {
                    finish(state, "error", format!("quota_data_missing: {reason}")).await
                }
                Err(ProxyError::UsageHttp { status, body }) => {
                    finish(state, "error", format!("usage_http {status}: {body}")).await
                }
                Err(err) => finish(state, "error", err.to_string()).await,
            }
        }
        "token_usage_rollup" => match state.proxy.rollup_token_usage_stats().await {
            Ok((rows, last_ts)) => {
                let msg = match last_ts {
                    Some(ts) => format!("rows={rows} last_rollup_ts={ts}"),
                    None => format!("rows={rows} last_rollup_ts=none"),
                };
                finish(state, "success", msg).await;
            }
            Err(err) => finish(state, "error", err.to_string()).await,
        },
        "auth_token_logs_gc" => match state.proxy.gc_auth_token_logs().await {
            Ok(deleted) => finish(state, "success", format!("deleted_rows={deleted}")).await,
            Err(err) => finish(state, "error", err.to_string()).await,
        },
        "mcp_sessions_gc" => match state.proxy.gc_mcp_sessions().await {
            Ok(deleted) => finish(state, "success", format!("deleted_rows={deleted}")).await,
            Err(err) => finish(state, "error", err.to_string()).await,
        },
        "mcp_session_init_backoffs_gc" => {
            match state.proxy.gc_mcp_session_init_backoffs().await {
                Ok(deleted) => finish(state, "success", format!("deleted_rows={deleted}")).await,
                Err(err) => finish(state, "error", err.to_string()).await,
            }
        },
        "request_logs_gc" => match state
            .proxy
            .gc_request_logs_with_options(scheduled_request_logs_gc_options())
            .await
        {
            Ok(report) => {
                let msg = format!(
                    "cleaned_bodies={} deleted_rows={} rollup_deleted={} completed={} retention_days={} batches={} elapsed_ms={}",
                    report.cleaned_request_log_bodies,
                    report.deleted_request_logs,
                    report.deleted_rollups,
                    report.completed,
                    report.retention_days,
                    report.batches,
                    report.elapsed_ms
                );
                finish(state, "success", msg).await;
            }
            Err(err) => finish(state, "error", err.to_string()).await,
        },
        "linuxdo_user_status_sync" => {
            run_linuxdo_user_status_sync_claimed_job(state, job_id).await;
        },
        "linuxdo_user_tag_binding_refresh" => {
            match state.proxy.refresh_linuxdo_user_tag_bindings().await {
                Ok(refreshed) => finish(state, "success", format!("refreshed={refreshed}")).await,
                Err(err) => finish(state, "error", err.to_string()).await,
            }
        },
        "forward_proxy_geo_refresh" => {
            match state
                .proxy
                .refresh_forward_proxy_geo_metadata(&state.api_key_ip_geo_origin, true)
                .await
            {
                Ok(refreshed) => {
                    finish(state, "success", format!("refreshed_candidates={refreshed}")).await
                }
                Err(err) => finish(state, "error", err.to_string()).await,
            }
        },
        "db_compaction" => {
            let _maintenance = db_maintenance_gate().write().await;
            match state.proxy.sqlite_db_stats().await {
                Ok(before) => match state.proxy.compact_sqlite_database().await {
                    Ok(after) => {
                        finish(
                            state,
                            "success",
                            format!(
                                "database_bytes_before={} database_bytes_after={} wal_bytes_before={} wal_bytes_after={} reclaimable_bytes_before={} reclaimable_bytes_after={} freelist_before={} freelist_after={}",
                                before.database_bytes,
                                after.database_bytes,
                                before.wal_bytes,
                                after.wal_bytes,
                                before.reclaimable_bytes,
                                after.reclaimable_bytes,
                                before.freelist_count,
                                after.freelist_count
                            ),
                        )
                        .await;
                    }
                    Err(err) => finish(state, "error", err.to_string()).await,
                }
                Err(err) => finish(state, "error", err.to_string()).await,
            }
        },
        _ => finish(state, "error", format!("unsupported manual job type: {job_type}")).await,
    }
}

fn spawn_db_compaction_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut next_allowed_at = Instant::now();
        loop {
            tokio::time::sleep(Duration::from_secs(3600)).await;
            if Instant::now() < next_allowed_at {
                continue;
            }
            let stats = match state.proxy.sqlite_db_stats().await {
                Ok(stats) => stats,
                Err(err) => {
                    eprintln!("db-compaction: stats error: {err}");
                    continue;
                }
            };
            if stats.reclaimable_bytes < db_compaction_min_reclaimable_bytes()
                || stats.reclaimable_ratio < db_compaction_min_reclaimable_ratio()
            {
                continue;
            }
            let Some(job_id) = claim_scheduled_job(
                state.as_ref(),
                "db_compaction",
                None,
                TRIGGER_SOURCE_AUTO,
                "db-compaction",
            )
            .await
            else {
                continue;
            };
            let run_state = state.clone();
            tokio::spawn(async move {
                run_manual_claimed_job(run_state, "db_compaction".to_string(), None, job_id).await;
            });
            next_allowed_at = Instant::now() + Duration::from_secs(db_compaction_cooldown_secs());
        }
    });
}

fn spawn_forward_proxy_geo_refresh_scheduler(state: Arc<AppState>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        loop {
            let wait_secs = state
                .proxy
                .forward_proxy_geo_refresh_wait_secs(twenty_four_hours_secs())
                .await;
            if wait_secs <= 0 {
                if state
                    .proxy
                    .forward_proxy_geo_refresh_due(twenty_four_hours_secs())
                    .await
                {
                    run_forward_proxy_geo_refresh_job(state.clone()).await;
                }
                tokio::time::sleep(Duration::from_secs(
                    forward_proxy_geo_refresh_recheck_secs() as u64,
                ))
                .await;
                continue;
            }

            let sleep_secs = wait_secs.min(forward_proxy_geo_refresh_recheck_secs()) as u64;
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
        }
    })
}

fn spawn_forward_proxy_maintenance_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            if let Err(err) = state.proxy.maybe_run_forward_proxy_maintenance().await {
                eprintln!("forward-proxy-maintenance: {err}");
            }
            tokio::time::sleep(Duration::from_secs(30)).await;
        }
    });
}
