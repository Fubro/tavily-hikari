use std::io::{self, Write};

use clap::Parser;
use dotenvy::dotenv;
use serde::Serialize;
use tavily_hikari::{
    HaOutboxGcChannelReport, HaOutboxGcOptions, HaOutboxGcReport,
    format_ha_outbox_gc_report_message, run_ha_outbox_gc_once,
};

#[derive(Debug, Parser)]
#[command(
    author,
    version,
    about = "Run bounded HA control outbox GC once, or repeatedly until complete"
)]
struct Cli {
    /// SQLite database path to mutate.
    #[arg(long, env = "PROXY_DB_PATH", default_value = "data/tavily_proxy.db")]
    db_path: String,

    /// Maximum ha_outbox rows to delete per batch.
    #[arg(long, default_value_t = HaOutboxGcOptions::default().batch_size, value_parser = positive_i64)]
    batch_size: i64,

    /// Maximum batches per GC pass.
    #[arg(long, default_value_t = HaOutboxGcOptions::default().max_batches, value_parser = positive_i64)]
    max_batches: i64,

    /// Maximum seconds per GC pass.
    #[arg(long, default_value_t = HaOutboxGcOptions::default().max_runtime_secs, value_parser = positive_u64)]
    max_runtime_secs: u64,

    /// Sleep between batches to reduce write pressure.
    #[arg(long, default_value_t = HaOutboxGcOptions::default().inter_batch_sleep_ms)]
    inter_batch_sleep_ms: u64,

    /// Continue running bounded passes until no retained control outbox rows remain.
    #[arg(long, default_value_t = false)]
    run_until_complete: bool,

    /// Emit JSON output. Plain output is retained for interactive use.
    #[arg(long, default_value_t = false)]
    json: bool,
}

fn positive_i64(value: &str) -> Result<i64, String> {
    let parsed = value
        .parse::<i64>()
        .map_err(|err| format!("expected a positive integer: {err}"))?;
    if parsed > 0 {
        Ok(parsed)
    } else {
        Err("expected a positive integer".to_string())
    }
}

fn positive_u64(value: &str) -> Result<u64, String> {
    let parsed = value
        .parse::<u64>()
        .map_err(|err| format!("expected a positive integer: {err}"))?;
    if parsed > 0 {
        Ok(parsed)
    } else {
        Err("expected a positive integer".to_string())
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CliReport {
    run_until_complete: bool,
    passes: usize,
    batch_size: i64,
    max_batches: i64,
    deleted_rows: i64,
    batches: i64,
    completed: bool,
    has_more: bool,
    channels: Vec<HaOutboxGcChannelReport>,
    wal_checkpoint_busy: bool,
    wal_checkpoint_log_frames: i64,
    wal_checkpoint_checkpointed_frames: i64,
    elapsed_ms: u128,
    pass_reports: Vec<HaOutboxGcReport>,
}

impl CliReport {
    fn from_passes(run_until_complete: bool, reports: Vec<HaOutboxGcReport>) -> Self {
        let last = reports
            .last()
            .expect("ha outbox cleanup cli always records at least one pass");
        Self {
            run_until_complete,
            passes: reports.len(),
            batch_size: last.batch_size,
            max_batches: last.max_batches,
            deleted_rows: reports.iter().map(|report| report.deleted_rows).sum(),
            batches: reports.iter().map(|report| report.batches).sum(),
            completed: last.completed,
            has_more: last.has_more,
            channels: last.channels.clone(),
            wal_checkpoint_busy: last.wal_checkpoint_busy,
            wal_checkpoint_log_frames: last.wal_checkpoint_log_frames,
            wal_checkpoint_checkpointed_frames: last.wal_checkpoint_checkpointed_frames,
            elapsed_ms: reports.iter().map(|report| report.elapsed_ms).sum(),
            pass_reports: reports,
        }
    }
}

fn write_json_report(mut writer: impl Write, report: &CliReport) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut writer, report)?;
    writer.write_all(b"\n")?;
    writer.flush()
}

fn write_plain_report(mut writer: impl Write, report: &CliReport) -> io::Result<()> {
    let aggregate = HaOutboxGcReport {
        batch_size: report.batch_size,
        max_batches: report.max_batches,
        deleted_rows: report.deleted_rows,
        batches: report.batches,
        completed: report.completed,
        has_more: report.has_more,
        channels: report.channels.clone(),
        wal_checkpoint_busy: report.wal_checkpoint_busy,
        wal_checkpoint_log_frames: report.wal_checkpoint_log_frames,
        wal_checkpoint_checkpointed_frames: report.wal_checkpoint_checkpointed_frames,
        elapsed_ms: report.elapsed_ms,
    };
    writeln!(
        writer,
        "ha_outbox_gc: {}",
        format_ha_outbox_gc_report_message(&aggregate, report.passes)
    )?;
    writer.flush()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    let cli = Cli::parse();
    let options = HaOutboxGcOptions {
        batch_size: cli.batch_size,
        max_batches: cli.max_batches,
        max_runtime_secs: cli.max_runtime_secs,
        inter_batch_sleep_ms: cli.inter_batch_sleep_ms,
    };
    let mut reports = Vec::new();

    loop {
        let report = run_ha_outbox_gc_once(&cli.db_path, options).await?;
        let completed = report.completed;
        reports.push(report);
        if completed || !cli.run_until_complete {
            break;
        }
    }

    let cli_report = CliReport::from_passes(cli.run_until_complete, reports);
    if cli.json {
        write_json_report(io::stdout().lock(), &cli_report)?;
    } else {
        write_plain_report(io::stdout().lock(), &cli_report)?;
    }

    Ok(())
}
