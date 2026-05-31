impl KeyStore {
    pub(crate) async fn restore_ha_snapshot_file(
        &self,
        snapshot_path: &std::path::Path,
    ) -> Result<usize, ProxyError> {
        let snapshot = snapshot_path.to_string_lossy().replace('\'', "''");
        let mut conn = self.pool.acquire().await?;
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *conn)
            .await?;
        sqlx::query(&format!("ATTACH DATABASE '{snapshot}' AS ha_snapshot"))
            .execute(&mut *conn)
            .await?;

        let tables: Vec<String> = sqlx::query_scalar(
            r#"
            SELECT name
              FROM ha_snapshot.sqlite_master
             WHERE type = 'table'
               AND name NOT LIKE 'sqlite_%'
               AND name NOT LIKE 'ha_%'
             ORDER BY name ASC
            "#,
        )
        .fetch_all(&mut *conn)
        .await?;

        sqlx::query("BEGIN IMMEDIATE").execute(&mut *conn).await?;
        for table in &tables {
            let ident = quote_sqlite_identifier(table);
            sqlx::query(&format!("DELETE FROM main.{ident}"))
                .execute(&mut *conn)
                .await?;
            sqlx::query(&format!(
                "INSERT INTO main.{ident} SELECT * FROM ha_snapshot.{ident}"
            ))
            .execute(&mut *conn)
            .await?;
        }
        sqlx::query("COMMIT").execute(&mut *conn).await?;
        sqlx::query("DETACH DATABASE ha_snapshot")
            .execute(&mut *conn)
            .await?;
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .await?;
        Ok(tables.len())
    }

    pub(crate) async fn persist_ha_node_state(
        &self,
        node_id: &str,
        role: HaNodeRole,
        edgeone_origin: Option<&str>,
        message: Option<&str>,
    ) -> Result<(), ProxyError> {
        sqlx::query(
            r#"
            INSERT INTO ha_node_state (
                id, node_id, role, edgeone_origin, message, updated_at
            )
            VALUES ('local', ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                node_id = excluded.node_id,
                role = excluded.role,
                edgeone_origin = excluded.edgeone_origin,
                message = excluded.message,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(node_id)
        .bind(role.as_str())
        .bind(edgeone_origin)
        .bind(message)
        .bind(Utc::now().timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn get_persisted_ha_node_role(&self) -> Result<Option<HaNodeRole>, ProxyError> {
        let raw: Option<String> =
            sqlx::query_scalar("SELECT role FROM ha_node_state WHERE id = 'local'")
                .fetch_optional(&self.pool)
                .await?;
        Ok(raw.as_deref().and_then(parse_ha_node_role))
    }

    pub(crate) async fn persist_ha_sync_watermark(
        &self,
        name: &str,
        source_node_id: Option<&str>,
        target_node_id: Option<&str>,
        watermark: i64,
        detail: Option<&str>,
    ) -> Result<(), ProxyError> {
        sqlx::query(
            r#"
            INSERT INTO ha_sync_watermarks (
                name, source_node_id, target_node_id, watermark, updated_at, detail
            )
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(name) DO UPDATE SET
                source_node_id = excluded.source_node_id,
                target_node_id = excluded.target_node_id,
                watermark = excluded.watermark,
                updated_at = excluded.updated_at,
                detail = excluded.detail
            "#,
        )
        .bind(name)
        .bind(source_node_id)
        .bind(target_node_id)
        .bind(watermark)
        .bind(Utc::now().timestamp())
        .bind(detail)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_ha_failover_operation(
        &self,
        record: &HaFailoverOperationRecord,
    ) -> Result<(), ProxyError> {
        let now = Utc::now().timestamp();
        sqlx::query(
            r#"
            INSERT INTO ha_failover_operations (
                id, operation_kind, target_node_id, from_origin, to_origin, status,
                message, created_at, updated_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                message = excluded.message,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(&record.operation_id)
        .bind(&record.operation_kind)
        .bind(record.target_node_id.as_deref())
        .bind(record.from_origin.as_deref())
        .bind(record.to_origin.as_deref())
        .bind(&record.status)
        .bind(record.message.as_deref())
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn insert_ha_edgeone_audit_log(
        &self,
        id: &str,
        action: &str,
        request_json: Option<&str>,
        response_json: Option<&str>,
        status: &str,
        message: Option<&str>,
    ) -> Result<(), ProxyError> {
        sqlx::query(
            r#"
            INSERT INTO ha_edgeone_audit_logs (
                id, action, request_json, response_json, status, message, created_at
            )
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(id)
        .bind(action)
        .bind(request_json)
        .bind(response_json)
        .bind(status)
        .bind(message)
        .bind(Utc::now().timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn claim_ha_recovery_batch(
        &self,
        batch_id: &str,
        source_node_id: &str,
        event_count: i64,
        checksum: &str,
    ) -> Result<bool, ProxyError> {
        let now = Utc::now().timestamp();
        let result = sqlx::query(
            r#"
            INSERT OR IGNORE INTO ha_recovery_batches (
                id, source_node_id, status, event_count, created_at, checksum
            )
            VALUES (?, ?, 'importing', ?, ?, ?)
            "#,
        )
        .bind(batch_id)
        .bind(source_node_id)
        .bind(event_count)
        .bind(now)
        .bind(checksum)
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() > 0)
    }

    pub(crate) async fn complete_ha_recovery_batch(
        &self,
        batch_id: &str,
        status: &str,
        event_count: i64,
    ) -> Result<(), ProxyError> {
        sqlx::query(
            r#"
            UPDATE ha_recovery_batches
               SET status = ?, event_count = ?, imported_at = ?
             WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(event_count)
        .bind(Utc::now().timestamp())
        .bind(batch_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub(crate) async fn import_ha_recovery_events(
        &self,
        request_logs: &[serde_json::Value],
        auth_token_logs: &[serde_json::Value],
    ) -> Result<i64, ProxyError> {
        let mut imported = 0_i64;
        for row in request_logs {
            imported += insert_json_row(
                &self.pool,
                "request_logs",
                &[
                    "api_key_id",
                    "auth_token_id",
                    "request_user_id",
                    "method",
                    "path",
                    "query",
                    "status_code",
                    "tavily_status_code",
                    "error_message",
                    "result_status",
                    "request_kind_key",
                    "request_kind_label",
                    "request_kind_detail",
                    "business_credits",
                    "failure_kind",
                    "key_effect_code",
                    "key_effect_summary",
                    "binding_effect_code",
                    "binding_effect_summary",
                    "selection_effect_code",
                    "selection_effect_summary",
                    "gateway_mode",
                    "experiment_variant",
                    "proxy_session_id",
                    "routing_subject_hash",
                    "upstream_operation",
                    "fallback_reason",
                    "request_body",
                    "response_body",
                    "forwarded_headers",
                    "dropped_headers",
                    "remote_addr",
                    "client_ip",
                    "client_ip_source",
                    "client_ip_trusted",
                    "ip_headers",
                    "visibility",
                    "created_at",
                ],
                row,
            )
            .await?;
        }
        for row in auth_token_logs {
            imported += insert_json_row(
                &self.pool,
                "auth_token_logs",
                &[
                    "token_id",
                    "method",
                    "path",
                    "query",
                    "http_status",
                    "mcp_status",
                    "request_kind_key",
                    "request_kind_label",
                    "request_kind_detail",
                    "result_status",
                    "error_message",
                    "failure_kind",
                    "key_effect_code",
                    "key_effect_summary",
                    "binding_effect_code",
                    "binding_effect_summary",
                    "selection_effect_code",
                    "selection_effect_summary",
                    "gateway_mode",
                    "experiment_variant",
                    "proxy_session_id",
                    "routing_subject_hash",
                    "upstream_operation",
                    "fallback_reason",
                    "counts_business_quota",
                    "business_credits",
                    "billing_subject",
                    "billing_state",
                    "request_user_id",
                    "api_key_id",
                    "request_log_id",
                    "created_at",
                ],
                row,
            )
            .await?;
        }
        Ok(imported)
    }
}

fn quote_sqlite_identifier(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

async fn insert_json_row(
    pool: &sqlx::SqlitePool,
    table: &str,
    allowed_columns: &[&str],
    row: &serde_json::Value,
) -> Result<i64, ProxyError> {
    let Some(object) = row.as_object() else {
        return Err(ProxyError::Other(
            "HA recovery row must be a JSON object".to_string(),
        ));
    };
    let mut columns = Vec::new();
    let mut values = Vec::new();
    for column in allowed_columns {
        let camel = snake_to_camel(column);
        if let Some(value) = object.get(*column).or_else(|| object.get(camel.as_str())) {
            columns.push(*column);
            values.push(value.clone());
        }
    }
    if columns.is_empty() {
        return Err(ProxyError::Other(
            "HA recovery row has no allowed columns".to_string(),
        ));
    }

    let column_sql = columns
        .iter()
        .map(|column| quote_sqlite_identifier(column))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = std::iter::repeat_n("?", columns.len())
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {} ({column_sql}) VALUES ({placeholders})",
        quote_sqlite_identifier(table)
    );
    let mut query = sqlx::query(&sql);
    for value in &values {
        query = bind_json_value(query, value);
    }
    query.execute(pool).await?;
    Ok(1)
}

fn bind_json_value<'q>(
    query: sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>>,
    value: &'q serde_json::Value,
) -> sqlx::query::Query<'q, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'q>> {
    match value {
        serde_json::Value::Null => query.bind(Option::<String>::None),
        serde_json::Value::Bool(value) => query.bind(i64::from(*value)),
        serde_json::Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                query.bind(value)
            } else if let Some(value) = value.as_u64().and_then(|value| i64::try_from(value).ok()) {
                query.bind(value)
            } else if let Some(value) = value.as_f64() {
                query.bind(value)
            } else {
                query.bind(value.to_string())
            }
        }
        serde_json::Value::String(value) => query.bind(value.as_str()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => query.bind(value.to_string()),
    }
}

fn snake_to_camel(value: &str) -> String {
    let mut out = String::new();
    let mut upper_next = false;
    for ch in value.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(ch.to_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn parse_ha_node_role(value: &str) -> Option<HaNodeRole> {
    match value {
        "full_master" => Some(HaNodeRole::FullMaster),
        "provisional_master" => Some(HaNodeRole::ProvisionalMaster),
        "standby" => Some(HaNodeRole::Standby),
        "recovery" => Some(HaNodeRole::Recovery),
        _ => None,
    }
}
