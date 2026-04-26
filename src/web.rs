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

// View model for templates
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

        // Uptime: up_count / total, 6 decimal places, max 1.0
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

        // Average response time (only successful checks)
        let avg_response_ms = {
            let times: Vec<i64> = history.iter().filter_map(|h| h.response_time_ms).collect();
            if times.is_empty() {
                "N/A".to_string()
            } else {
                let avg = times.iter().sum::<i64>() / times.len() as i64;
                format!("{avg}ms")
            }
        };

        // SVG sparkline from response times (last 50 checks, oldest→newest)
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

/// Build an inline SVG sparkline from history entries.
/// X axis = time (oldest left), Y axis = response_time_ms.
/// Down checks shown as red dots. Max value label on Y axis.
fn build_sparkline(history: &[History]) -> String {
    let points: Vec<&History> = {
        let start = if history.len() > 50 {
            history.len() - 50
        } else {
            0
        };
        history[start..].iter().collect()
    };

    if points.is_empty() {
        return String::new();
    }

    let w = 260.0_f64; // extra 60px left for y-axis labels
    let h = 48.0_f64;
    let plot_x = 36.0_f64; // left margin for labels
    let plot_w = w - plot_x;
    let n = points.len();

    let max_val = points
        .iter()
        .filter_map(|p| p.response_time_ms.map(|v| v as f64))
        .fold(1.0_f64, f64::max);

    let coords: Vec<String> = points
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let x = plot_x
                + if n == 1 {
                    plot_w / 2.0
                } else {
                    i as f64 * plot_w / (n - 1) as f64
                };
            let y = match p.response_time_ms {
                Some(ms) => h - (ms as f64 / max_val * (h - 6.0)) - 3.0,
                None => 3.0,
            };
            format!("{:.1},{:.1}", x, y)
        })
        .collect();

    let dots: String = points
        .iter()
        .enumerate()
        .filter(|(_, p)| matches!(p.status, crate::models::HistoryStatus::Down))
        .map(|(i, _)| {
            let x = plot_x
                + if n == 1 {
                    plot_w / 2.0
                } else {
                    i as f64 * plot_w / (n - 1) as f64
                };
            format!(
                "<circle cx=\"{:.1}\" cy=\"3\" r=\"2.5\" fill=\"#ef4444\"/>",
                x
            )
        })
        .collect();

    // Y axis: max label top, "0" bottom
    let max_label = if max_val >= 1000.0 {
        format!("{:.0}s", max_val / 1000.0)
    } else {
        format!("{:.0}", max_val)
    };

    let pts = coords.join(" ");
    format!(
        "<svg viewBox=\"0 0 {w} {h}\" xmlns=\"http://www.w3.org/2000/svg\" style=\"width:100%;height:{h}px\">\
          <text x=\"{lx}\" y=\"8\" font-size=\"7\" fill=\"#00ff4160\" text-anchor=\"end\" font-family=\"monospace\">{max_label}</text>\
          <text x=\"{lx}\" y=\"{bot}\" font-size=\"7\" fill=\"#00ff4160\" text-anchor=\"end\" font-family=\"monospace\">0</text>\
          <line x1=\"{plot_x}\" y1=\"3\" x2=\"{plot_x}\" y2=\"{h}\" stroke=\"#00ff4120\" stroke-width=\"1\"/>\
          <polyline points=\"{pts}\" fill=\"none\" stroke=\"#00ff41\" stroke-width=\"1.5\" stroke-linejoin=\"round\" opacity=\"0.85\"/>\
          {dots}\
        </svg>",
        w = w as i32, h = h as i32,
        lx = (plot_x - 2.0) as i32,
        bot = (h - 1.0) as i32,
        plot_x = plot_x as i32,
        max_label = max_label,
        pts = pts,
        dots = dots,
    )
}

// View model for history
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

// Template structs
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

// Form data for creating/updating monitor
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

// Handlers

/// Fetch all monitors with their last 100 history entries and build MonitorViews.
async fn fetch_monitor_views(
    pool: &DbPool,
) -> Result<Vec<MonitorView>, (axum::http::StatusCode, String)> {
    let monitors: Vec<Monitor> = sqlx::query_as::<_, Monitor>("SELECT * FROM monitors ORDER BY id")
        .fetch_all(pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut views = Vec::with_capacity(monitors.len());
    for m in monitors {
        let history: Vec<History> = sqlx::query_as::<_, History>(
            "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at ASC LIMIT 100",
        )
        .bind(m.id)
        .fetch_all(pool)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

        views.push(MonitorView::from_monitor_with_history(m, &history));
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

    let history: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at ASC LIMIT 100",
    )
    .bind(monitor.id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

    let history: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at ASC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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
    let histories: Vec<History> = sqlx::query_as::<_, History>(
        "SELECT * FROM history WHERE monitor_id = ? ORDER BY checked_at DESC LIMIT 100",
    )
    .bind(id)
    .fetch_all(&pool)
    .await
    .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

// Router
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
        .route("/monitors", post(create_monitor))
        .route("/clear-modal", get(clear_modal))
        .nest_service("/static", ServeDir::new("static"))
        .with_state(pool)
}
