use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::Html,
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use tower_http::services::ServeDir;

use crate::db::DbPool;
use crate::models::{History, Monitor};
use crate::services::checker::Checkable;

#[allow(dead_code)]
struct MonitorView {
    id: i64,
    name: String,
    url: String,
    interval_seconds: i64,
    status: String,
    last_check: String,
    uptime: f64, // 0.0 – 1.0, or -1.0 if no data
    uptime_str: String,
    avg_response_ms: String,
    sparkline: String,
}

impl MonitorView {
    fn from_monitor_with_history(monitor: Monitor, history: &[History]) -> Self {
        let status = monitor.status.to_string();
        let last_check = monitor
            .last_check_at
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Never".to_string());

        let (uptime, uptime_str) = if history.is_empty() {
            (-1.0_f64, "N/A".to_string())
        } else {
            let up = history
                .iter()
                .filter(|h| matches!(h.status, crate::models::HistoryStatus::Up))
                .count();
            let ratio = (up as f64 / history.len() as f64).min(1.0);
            (ratio, format!("{:.6}", ratio))
        };

        let avg_response_ms = {
            let times: Vec<i64> = history.iter().filter_map(|h| h.response_time_ms).collect();
            if times.is_empty() {
                "N/A".to_string()
            } else {
                let avg = times.iter().sum::<i64>() / times.len() as i64;
                format!("{avg}ms")
            }
        };

        let sparkline = build_sparkline(history);

        MonitorView {
            id: monitor.id,
            name: monitor.name,
            url: monitor.url,
            interval_seconds: monitor.check_interval_secs,
            status,
            last_check,
            uptime,
            uptime_str,
            avg_response_ms,
            sparkline,
        }
    }
}

/// Build an ASCII art vertical bar chart from history entries.
///
/// Renders up to 100 of the most recent checks. Each check = one column.
/// Column width = 1 char, separated by 1 space → total char width = 2N-1.
///
/// The <pre> carries `--chart-char-count` as a CSS custom property so the
/// stylesheet scales font-size to exactly fill the card width — no JS needed.
fn build_sparkline(history: &[History]) -> String {
    const ROWS: usize = 6;
    const MAX_COLS: usize = 100;

    let points: Vec<&History> = {
        let start = if history.len() > MAX_COLS {
            history.len() - MAX_COLS
        } else {
            0
        };
        history[start..].iter().collect()
    };

    if points.is_empty() {
        return String::new();
    }

    let n = points.len();

    let max_val = points
        .iter()
        .filter_map(|p| p.response_time_ms.map(|v| v as f64))
        .fold(1.0_f64, f64::max);

    struct Col {
        height: usize,
        down: bool,
    }
    let cols: Vec<Col> = points
        .iter()
        .map(|p| {
            let down = matches!(p.status, crate::models::HistoryStatus::Down);
            let height = match p.response_time_ms {
                Some(ms) => {
                    let ratio = ms as f64 / max_val;
                    ((ratio * ROWS as f64).round() as usize).max(1).min(ROWS)
                }
                None => ROWS,
            };
            Col { height, down }
        })
        .collect();

    // Bar rows (row 0 = top/tallest)
    let mut lines: Vec<String> = (0..ROWS)
        .map(|row| {
            let threshold = ROWS - row;
            let mut line = String::new();
            for (i, c) in cols.iter().enumerate() {
                if i > 0 {
                    line.push(' ');
                }
                if c.height >= threshold {
                    if c.down {
                        line.push_str("<span style=\"color:#ef4444\">!</span>");
                    } else {
                        line.push('\u{2588}'); // █
                    }
                } else {
                    line.push(' ');
                }
            }
            line
        })
        .collect();

    // Baseline ─ / ┴ (red under DOWN)
    let baseline: String = cols
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let sep = if i > 0 { "\u{2500}" } else { "" }; // ─
            if c.down {
                format!("{}<span style=\"color:#ef4444\">\u{2534}</span>", sep) // ┴
            } else {
                format!("{}\u{2500}", sep) // ─
            }
        })
        .collect();
    lines.push(baseline);

    // Timestamp row
    fn fmt_ts(dt: Option<chrono::DateTime<chrono::Utc>>) -> String {
        match dt {
            Some(d) => d.format("%H:%M:%S").to_string(),
            None => String::new(),
        }
    }
    let ts_first = fmt_ts(points.first().and_then(|p| p.checked_at));
    let ts_last = fmt_ts(points.last().and_then(|p| p.checked_at));

    if !ts_first.is_empty() || !ts_last.is_empty() {
        let chart_chars = 2 * n - 1;
        let used = ts_first.len() + ts_last.len();
        let padding = if chart_chars > used {
            chart_chars - used
        } else {
            1
        };
        lines.push(format!(
            "<span style=\"color:#22c55e55\">{}{}{}</span>",
            ts_first,
            " ".repeat(padding),
            ts_last,
        ));
    }

    // CSS custom property tells the stylesheet how many chars wide this chart is,
    // allowing it to scale font-size so the chart fills the card width exactly.
    let char_count = 2 * n - 1;

    format!(
        "<pre class=\"ascii-chart\" style=\"--chart-char-count:{char_count}\">{content}</pre>",
        char_count = char_count,
        content = lines.join("\n"),
    )
}

struct HistoryView {
    status: String,
    response_time_ms: Option<i64>,
    error_message: Option<String>,
    checked_at: Option<String>,
}

impl From<History> for HistoryView {
    fn from(h: History) -> Self {
        let status = match h.status {
            crate::models::HistoryStatus::Up => "up".to_string(),
            crate::models::HistoryStatus::Down => "down".to_string(),
        };

        HistoryView {
            status,
            response_time_ms: h.response_time_ms,
            error_message: h.error_message,
            checked_at: h
                .checked_at
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string()),
        }
    }
}

#[derive(Template)]
#[template(path = "monitors.html")]
struct MonitorsTemplate {
    monitors: Vec<MonitorView>,
    total: usize,
    online: usize,
    offline: usize,
    pending: usize,
}

#[derive(Template)]
#[template(path = "components/stats.html")]
struct StatsTemplate {
    total: usize,
    online: usize,
    offline: usize,
    pending: usize,
}

#[derive(Template)]
#[template(path = "components/monitor_grid.html")]
struct MonitorGridTemplate {
    monitors: Vec<MonitorView>,
}

#[derive(Template)]
#[template(path = "components/new_monitor_form.html")]
struct NewMonitorFormTemplate;

#[derive(Template)]
#[template(path = "components/edit_monitor_form.html")]
struct EditMonitorFormTemplate {
    monitor: MonitorView,
}

#[derive(Template)]
#[template(path = "components/monitor_history.html")]
struct MonitorHistoryTemplate {
    histories: Vec<HistoryView>,
}

#[derive(Deserialize)]
pub struct CreateMonitorForm {
    pub name: String,
    pub url: String,
    pub interval_seconds: u64,
}

#[derive(Deserialize)]
pub struct UpdateMonitorForm {
    pub name: String,
    pub url: String,
    pub interval_seconds: u64,
}

async fn fetch_monitor_views(
    pool: &DbPool,
) -> Result<Vec<MonitorView>, (axum::http::StatusCode, String)> {
    let monitors: Vec<Monitor> = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors ORDER BY id")
        .fetch_all(pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut views = Vec::with_capacity(monitors.len());

    for monitor in monitors {
        let mut history: Vec<History> = sqlx::query_as::<_, History>(
            "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 100",
        )
        .bind(monitor.id)
        .fetch_all(pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        history.reverse();

        views.push(MonitorView::from_monitor_with_history(monitor, &history));
    }

    Ok(views)
}

async fn index(
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let monitors = fetch_monitor_views(&pool).await?;

    let total = monitors.len();
    let online = monitors.iter().filter(|m| m.status == "up").count();
    let offline = monitors.iter().filter(|m| m.status == "down").count();
    let pending = monitors.iter().filter(|m| m.status == "unknown").count();

    let template = MonitorsTemplate {
        monitors,
        total,
        online,
        offline,
        pending,
    };

    Ok(Html(template.render().unwrap()))
}

async fn partial_stats(
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let monitors = fetch_monitor_views(&pool).await?;

    let total = monitors.len();
    let online = monitors.iter().filter(|m| m.status == "up").count();
    let offline = monitors.iter().filter(|m| m.status == "down").count();
    let pending = monitors.iter().filter(|m| m.status == "unknown").count();

    let template = StatsTemplate {
        total,
        online,
        offline,
        pending,
    };

    Ok(Html(template.render().unwrap()))
}

async fn partial_monitors(
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let monitors = fetch_monitor_views(&pool).await?;
    let template = MonitorGridTemplate { monitors };

    Ok(Html(template.render().unwrap()))
}

async fn new_monitor_form() -> Html<String> {
    let template = NewMonitorFormTemplate;

    Html(template.render().unwrap())
}

async fn edit_monitor_form(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Monitor not found".to_string(),
            )
        })?;

    let mut history: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 100",
    )
    .bind(monitor.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    history.reverse();

    let monitor_view = MonitorView::from_monitor_with_history(monitor, &history);
    let template = EditMonitorFormTemplate {
        monitor: monitor_view,
    };

    Ok(Html(template.render().unwrap()))
}

async fn update_monitor(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
    Form(form): Form<UpdateMonitorForm>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    sqlx::query(
        r#"
        UPDATE monitors
        SET name = ?, url = ?, check_interval_secs = ?, updated_at = datetime('now')
        WHERE id = ?
        "#,
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(form.interval_seconds as i64)
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_one(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut history: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    history.reverse();

    let monitor_view = MonitorView::from_monitor_with_history(monitor, &history);
    let template = EditMonitorFormTemplate {
        monitor: monitor_view,
    };

    Ok(Html(template.render().unwrap()))
}

async fn monitor_history(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let mut histories: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    histories.reverse();

    let histories: Vec<HistoryView> = histories.into_iter().map(HistoryView::from).collect();
    let template = MonitorHistoryTemplate { histories };

    Ok(Html(template.render().unwrap()))
}

async fn create_monitor(
    State(pool): State<DbPool>,
    Form(form): Form<CreateMonitorForm>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    sqlx::query(
        r#"
        INSERT INTO monitors (name, url, check_interval_secs)
        VALUES (?, ?, ?)
        "#,
    )
    .bind(&form.name)
    .bind(&form.url)
    .bind(form.interval_seconds as i64)
    .execute(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let monitors = fetch_monitor_views(&pool).await?;
    let template = MonitorGridTemplate { monitors };

    Ok(Html(template.render().unwrap()))
}

async fn delete_monitor(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    sqlx::query("DELETE FROM history WHERE monitor_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query("DELETE FROM monitors WHERE id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let monitors = fetch_monitor_views(&pool).await?;
    let template = MonitorGridTemplate { monitors };

    Ok(Html(template.render().unwrap()))
}

async fn clear_modal() -> Html<&'static str> {
    Html("")
}

async fn clear_history(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    sqlx::query("DELETE FROM history WHERE monitor_id = ?")
        .bind(id)
        .execute(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    sqlx::query(
        "UPDATE monitors SET status = 'unknown', last_check_at = NULL, last_response_time_ms = NULL, updated_at = datetime('now') WHERE id = ?",
    )
    .bind(id)
    .execute(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    render_single_monitor_card(id, &pool).await
}

async fn check_now(
    Path(id): Path<i64>,
    State(pool): State<DbPool>,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Monitor not found".to_string(),
            )
        })?;

    let checker = crate::services::http_checker::HttpChecker::new(
        monitor.id,
        monitor.name.clone(),
        monitor.url.clone(),
        monitor.timeout_secs as u64,
    );

    let result = checker
        .check(&pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    crate::services::checker::record_check_result(&pool, id, &result)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    render_single_monitor_card(id, &pool).await
}

async fn render_single_monitor_card(
    id: i64,
    pool: &DbPool,
) -> Result<Html<String>, (axum::http::StatusCode, String)> {
    let monitor = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or_else(|| {
            (
                axum::http::StatusCode::NOT_FOUND,
                "Monitor not found".to_string(),
            )
        })?;

    let mut history: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    history.reverse();

    let view = MonitorView::from_monitor_with_history(monitor, &history);

    #[derive(askama::Template)]
    #[template(path = "components/monitor_card.html")]
    struct CardTemplate {
        monitor: MonitorView,
    }

    let html = CardTemplate { monitor: view }
        .render()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Html(html))
}

pub fn web_router(pool: DbPool) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/partial/stats", get(partial_stats))
        .route("/partial/monitors", get(partial_monitors))
        .route("/monitors/new", get(new_monitor_form))
        .route("/monitors/{id}/edit", get(edit_monitor_form))
        .route(
            "/monitors/{id}",
            post(update_monitor).delete(delete_monitor),
        )
        .route("/monitors/{id}/history", get(monitor_history))
        .route(
            "/monitors/{id}/history/clear",
            axum::routing::delete(clear_history),
        )
        .route("/monitors/{id}/check", post(check_now))
        .route("/monitors", post(create_monitor))
        .route("/clear-modal", get(clear_modal))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(pool)
}
