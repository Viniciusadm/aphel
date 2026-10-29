use std::collections::BTreeMap;
use std::fmt::Write;

use crate::application::system::model::JobKindCounts;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetricsSnapshot {
    pub version: String,
    pub database_up: bool,
    pub jobs: Vec<JobKindCounts>,
    pub oldest_ready_age_seconds: Option<i64>,
    pub scheduler_overdue_seconds: Option<i64>,
    pub worker_heartbeat_age_seconds: Option<i64>,
    pub workers_alive: u64,
    pub http_responses: Vec<(&'static str, u64)>,
}

pub fn render_prometheus(snapshot: &MetricsSnapshot) -> String {
    let mut output = String::new();
    family(
        &mut output,
        "aphel_build_info",
        "gauge",
        "Aphel build information.",
    );
    sample(
        &mut output,
        "aphel_build_info",
        &[("version", &snapshot.version)],
        1,
    );

    family(
        &mut output,
        "aphel_database_up",
        "gauge",
        "Whether the database answered the last probe.",
    );
    sample(
        &mut output,
        "aphel_database_up",
        &[],
        u64::from(snapshot.database_up),
    );

    family(
        &mut output,
        "aphel_jobs",
        "gauge",
        "Background jobs by queue and state.",
    );
    let mut by_queue: BTreeMap<&str, [u64; 4]> = BTreeMap::new();
    for entry in &snapshot.jobs {
        let totals = by_queue.entry(entry.queue.as_str()).or_default();
        totals[0] += entry.ready;
        totals[1] += entry.scheduled;
        totals[2] += entry.running;
        totals[3] += entry.failed;
    }
    for (queue, totals) in &by_queue {
        for (state, value) in ["ready", "scheduled", "running", "failed"]
            .iter()
            .zip(totals)
        {
            sample(
                &mut output,
                "aphel_jobs",
                &[("queue", queue), ("state", state)],
                *value,
            );
        }
    }

    family(
        &mut output,
        "aphel_jobs_failed_by_kind",
        "gauge",
        "Jobs that exhausted their attempts, by kind.",
    );
    let mut failed_by_kind: BTreeMap<&str, u64> = BTreeMap::new();
    for entry in snapshot.jobs.iter().filter(|entry| entry.failed > 0) {
        *failed_by_kind.entry(entry.kind.as_str()).or_default() += entry.failed;
    }
    for (kind, value) in &failed_by_kind {
        sample(
            &mut output,
            "aphel_jobs_failed_by_kind",
            &[("kind", kind)],
            *value,
        );
    }

    family(
        &mut output,
        "aphel_jobs_oldest_ready_age_seconds",
        "gauge",
        "Age of the oldest job ready to run (0 when the queue is empty).",
    );
    sample(
        &mut output,
        "aphel_jobs_oldest_ready_age_seconds",
        &[],
        non_negative(snapshot.oldest_ready_age_seconds.unwrap_or(0)),
    );

    if let Some(overdue) = snapshot.scheduler_overdue_seconds {
        family(
            &mut output,
            "aphel_scheduler_overdue_seconds",
            "gauge",
            "How late the most overdue recurring schedule is.",
        );
        sample(
            &mut output,
            "aphel_scheduler_overdue_seconds",
            &[],
            non_negative(overdue),
        );
    }

    family(
        &mut output,
        "aphel_workers_alive",
        "gauge",
        "Workers with a recent heartbeat.",
    );
    sample(
        &mut output,
        "aphel_workers_alive",
        &[],
        snapshot.workers_alive,
    );

    if let Some(age) = snapshot.worker_heartbeat_age_seconds {
        family(
            &mut output,
            "aphel_worker_heartbeat_age_seconds",
            "gauge",
            "Seconds since the most recent worker heartbeat.",
        );
        sample(
            &mut output,
            "aphel_worker_heartbeat_age_seconds",
            &[],
            non_negative(age),
        );
    }

    family(
        &mut output,
        "aphel_http_responses_total",
        "counter",
        "HTTP responses served by this process, by status class.",
    );
    for (class, value) in &snapshot.http_responses {
        sample(
            &mut output,
            "aphel_http_responses_total",
            &[("class", class)],
            *value,
        );
    }
    output
}

fn family(output: &mut String, name: &str, kind: &str, help: &str) {
    let _ = writeln!(output, "# HELP {name} {help}");
    let _ = writeln!(output, "# TYPE {name} {kind}");
}

fn sample(output: &mut String, name: &str, labels: &[(&str, &str)], value: u64) {
    output.push_str(name);
    if !labels.is_empty() {
        output.push('{');
        for (index, (label, raw)) in labels.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            let _ = write!(output, "{label}=\"{}\"", escape_label(raw));
        }
        output.push('}');
    }
    let _ = writeln!(output, " {value}");
}

fn escape_label(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn non_negative(value: i64) -> u64 {
    value.max(0) as u64
}
