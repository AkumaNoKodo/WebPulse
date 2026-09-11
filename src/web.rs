use askama::Template;
use axum::{
    extract::{Form, Path, State},
    response::Html,
    routing::{get, post},
    Router,
};
use tower_http::services::ServeDir;

use crate::error::WebError;
use crate::models::{CreateMonitor, History, HistoryStatus, Monitor, UpdateMonitor};
use crate::services::checker::probe;
use crate::store::{self, AppState, Stats};

type WebResult = Result<Html<String>, WebError>;

#[derive(Debug)]
struct MonitorView {
    id: i64,
    name: String,
    url: String,
    check_interval_secs: i64,
    timeout_secs: i64,
    status: &'static str,
    last_check: String,
    uptime: f64,
    uptime_str: String,
    avg_response_ms: String,
    sparkline: String,
}

impl MonitorView {
    fn new(monitor: Monitor, history: &[History]) -> Self {
        let (uptime, uptime_str) = if history.is_empty() {
            (-1.0, "N/A".to_string())
        } else {
            let up = history
                .iter()
                .filter(|h| h.status == HistoryStatus::Up)
                .count();
            let ratio = up as f64 / history.len() as f64;
            (ratio, format!("{:.2}%", ratio * 100.0))
        };

        let response_times: Vec<i64> = history.iter().filter_map(|h| h.response_time_ms).collect();
        let avg_response_ms = if response_times.is_empty() {
            "N/A".to_string()
        } else {
            let average = response_times.iter().sum::<i64>() / response_times.len() as i64;
            format!("{average}ms")
        };

        MonitorView {
            id: monitor.id,
            name: monitor.name,
            url: monitor.url,
            check_interval_secs: monitor.check_interval_secs,
            timeout_secs: monitor.timeout_secs,
            status: monitor.status.as_str(),
            last_check: monitor
                .last_check_at
                .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                .unwrap_or_else(|| "Never".to_string()),
            uptime,
            uptime_str,
            avg_response_ms,
            sparkline: build_sparkline(history),
        }
    }
}

/// ASCII bar chart of the recent response times, one column per check.
///
/// The `<pre>` carries `--chart-char-count` so the stylesheet can scale font-size
/// to fill the card width exactly, which keeps this free of client-side script.
fn build_sparkline(history: &[History]) -> String {
    const ROWS: usize = 6;

    if history.is_empty() {
        return String::new();
    }

    let max_response_time = history
        .iter()
        .filter_map(|h| h.response_time_ms.map(|ms| ms as f64))
        .fold(1.0_f64, f64::max);

    let columns: Vec<(usize, bool)> = history
        .iter()
        .map(|entry| {
            let height = match entry.response_time_ms {
                Some(ms) => (((ms as f64 / max_response_time) * ROWS as f64).round() as usize)
                    .clamp(1, ROWS),
                None => ROWS,
            };
            (height, entry.status == HistoryStatus::Down)
        })
        .collect();

    let mut lines: Vec<String> = (0..ROWS)
        .map(|row| {
            let threshold = ROWS - row;
            columns
                .iter()
                .map(|&(height, down)| match (height >= threshold, down) {
                    (false, _) => " ".to_string(),
                    (true, true) => "<span style=\"color:#ef4444\">!</span>".to_string(),
                    (true, false) => "\u{2588}".to_string(),
                })
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();

    lines.push(
        columns
            .iter()
            .map(|&(_, down)| {
                if down {
                    "<span style=\"color:#ef4444\">\u{2534}</span>".to_string()
                } else {
                    "\u{2500}".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\u{2500}"),
    );

    let char_count = 2 * history.len() - 1;
    let first = history[0].checked_at.format("%H:%M:%S").to_string();
    let last = history[history.len() - 1]
        .checked_at
        .format("%H:%M:%S")
        .to_string();
    let padding = char_count.saturating_sub(first.len() + last.len()).max(1);

    lines.push(format!(
        "<span style=\"color:#22c55e55\">{first}{}{last}</span>",
        " ".repeat(padding)
    ));

    format!(
        "<pre class=\"ascii-chart\" style=\"--chart-char-count:{char_count}\">{}</pre>",
        lines.join("\n")
    )
}

#[derive(Template)]
#[template(path = "monitors.html")]
struct MonitorsTemplate {
    stats: Stats,
    monitors: Vec<MonitorView>,
}

#[derive(Template)]
#[template(path = "components/stats.html")]
struct StatsTemplate {
    stats: Stats,
}

#[derive(Template)]
#[template(path = "components/monitor_grid.html")]
struct MonitorGridTemplate {
    monitors: Vec<MonitorView>,
}

#[derive(Template)]
#[template(path = "components/monitor_card.html")]
struct MonitorCardTemplate {
    monitor: MonitorView,
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
    name: String,
    entries: Vec<History>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(index))
        .route("/partial/stats", get(partial_stats))
        .route("/partial/monitors", get(partial_monitors))
        .route("/monitors", post(create_monitor))
        .route("/monitors/new", get(new_monitor_form))
        .route(
            "/monitors/{id}",
            post(update_monitor).delete(delete_monitor),
        )
        .route("/monitors/{id}/edit", get(edit_monitor_form))
        .route("/monitors/{id}/check", post(check_now))
        .route(
            "/monitors/{id}/history",
            get(monitor_history).delete(clear_history),
        )
        .route("/clear-modal", get(clear_modal))
        .nest_service("/static", ServeDir::new("static"))
}

async fn index(State(state): State<AppState>) -> WebResult {
    render(MonitorsTemplate {
        stats: store::stats(&state).await?,
        monitors: monitor_views(&state).await?,
    })
}

async fn partial_stats(State(state): State<AppState>) -> WebResult {
    render(StatsTemplate {
        stats: store::stats(&state).await?,
    })
}

async fn partial_monitors(State(state): State<AppState>) -> WebResult {
    render_grid(&state).await
}

async fn new_monitor_form() -> WebResult {
    render(NewMonitorFormTemplate)
}

async fn edit_monitor_form(State(state): State<AppState>, Path(id): Path<i64>) -> WebResult {
    render(EditMonitorFormTemplate {
        monitor: monitor_view(&state, id).await?,
    })
}

async fn create_monitor(
    State(state): State<AppState>,
    Form(input): Form<CreateMonitor>,
) -> WebResult {
    store::create_monitor(&state, input).await?;

    render_grid(&state).await
}

async fn update_monitor(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Form(input): Form<UpdateMonitor>,
) -> WebResult {
    store::update_monitor(&state, id, input).await?;

    render_grid(&state).await
}

async fn delete_monitor(State(state): State<AppState>, Path(id): Path<i64>) -> WebResult {
    store::delete_monitor(&state, id).await?;

    render_grid(&state).await
}

async fn monitor_history(State(state): State<AppState>, Path(id): Path<i64>) -> WebResult {
    let monitor = store::get_monitor(&state, id).await?;
    let mut entries = store::history(&state, id).await?;
    entries.reverse();

    render(MonitorHistoryTemplate {
        name: monitor.name,
        entries,
    })
}

async fn clear_history(State(state): State<AppState>, Path(id): Path<i64>) -> WebResult {
    store::clear_history(&state, id).await?;

    render_card(&state, id).await
}

async fn check_now(State(state): State<AppState>, Path(id): Path<i64>) -> WebResult {
    let monitor = store::get_monitor(&state, id).await?;
    let timeout = std::time::Duration::from_secs(monitor.timeout_secs.max(1) as u64);
    let result = probe(&state.http, &monitor.url, timeout).await;

    store::record_check(&state, id, &result).await?;

    render_card(&state, id).await
}

async fn clear_modal() -> Html<&'static str> {
    Html("")
}

async fn monitor_views(state: &AppState) -> Result<Vec<MonitorView>, WebError> {
    let monitors = store::list_monitors(state).await?;
    let mut views = Vec::with_capacity(monitors.len());

    for monitor in monitors {
        let history = store::history(state, monitor.id).await?;
        views.push(MonitorView::new(monitor, &history));
    }

    Ok(views)
}

async fn monitor_view(state: &AppState, id: i64) -> Result<MonitorView, WebError> {
    let monitor = store::get_monitor(state, id).await?;
    let history = store::history(state, id).await?;

    Ok(MonitorView::new(monitor, &history))
}

async fn render_grid(state: &AppState) -> WebResult {
    render(MonitorGridTemplate {
        monitors: monitor_views(state).await?,
    })
}

async fn render_card(state: &AppState, id: i64) -> WebResult {
    render(MonitorCardTemplate {
        monitor: monitor_view(state, id).await?,
    })
}

fn render(template: impl Template) -> WebResult {
    Ok(Html(template.render()?))
}
