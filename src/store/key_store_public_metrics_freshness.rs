impl KeyStore {
    async fn flush_request_stats_writes_if_public_metrics_stale(
        &self,
        month_start: i64,
        day_start: i64,
        day_end: i64,
    ) -> Result<(), ProxyError> {
        let Some(oldest_pending_created_at) =
            self.request_stats_coalescer.pending_oldest_created_at().await
        else {
            return Ok(());
        };
        let newest_pending_created_at = self
            .request_stats_coalescer
            .pending_newest_created_at()
            .await
            .unwrap_or(oldest_pending_created_at);

        let pending_overlaps_day =
            oldest_pending_created_at < day_end && newest_pending_created_at >= day_start;
        let pending_overlaps_month = newest_pending_created_at >= month_start;

        if !(pending_overlaps_day || pending_overlaps_month) {
            return Ok(());
        }

        self.flush_request_stats_writes().await?;
        Ok(())
    }
}
