use chrono::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use teloxide::{prelude::*, utils::command::BotCommands};
use tokio::sync::Mutex;

// Configuration structure
#[derive(Deserialize, Clone, Debug)]
struct Config {
    check_interval_seconds: u64,
    telegram_chat_id: i64,
    failure_threshold: u32,
    servers: Vec<String>,
    telegram_token: String,
}

// Server status tracking
struct ServerStatus {
    consecutive_failures: u32,
    last_notification: Option<DateTime<Utc>>,
}

// Statistics for each server
struct ServerStats {
    total_checks: u32,
    failed_checks: u32,
    last_failure: Option<DateTime<Utc>>,
}

// Application state
struct AppState {
    servers: HashMap<String, ServerStats>,
    status: HashMap<String, ServerStatus>,
    start_time: Instant,
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Available commands:")]
enum Command {
    #[command(description = "show availability statistics")]
    Uptime,
    #[command(description = "show statistics for specific server")]
    Stats(String),
}

async fn check_server(url: &str) -> Result<(), reqwest::Error> {
    let response = reqwest::get(url).await?;
    response.error_for_status()?;
    Ok(())
}

async fn monitor_server(bot: Bot, config: Arc<Config>, state: Arc<Mutex<AppState>>, url: String) {
    let mut interval = tokio::time::interval(Duration::from_secs(config.check_interval_seconds));

    loop {
        interval.tick().await;
        let check_result = check_server(&url).await;

        {
            let mut state_guard = state.lock().await;
            let server_stats =
                state_guard
                    .servers
                    .entry(url.clone())
                    .or_insert_with(|| ServerStats {
                        total_checks: 0,
                        failed_checks: 0,
                        last_failure: None,
                    });

            server_stats.total_checks += 1;

            if check_result.is_err() {
                server_stats.failed_checks += 1;
                server_stats.last_failure = Some(Utc::now());
            }
        }

        let should_notify = {
            let mut state_guard = state.lock().await;
            let server_status =
                state_guard
                    .status
                    .entry(url.clone())
                    .or_insert_with(|| ServerStatus {
                        consecutive_failures: 0,
                        last_notification: None,
                    });

            match check_result {
                Ok(_) => {
                    let was_failing =
                        server_status.consecutive_failures >= config.failure_threshold;
                    server_status.consecutive_failures = 0;
                    was_failing
                }
                Err(_) => {
                    server_status.consecutive_failures += 1;
                    server_status.consecutive_failures == config.failure_threshold
                }
            }
        };

        match (check_result, should_notify) {
            (Ok(_), true) => {
                let recovery_message = format!("Server {} is back online!", url);
                if let Err(e) = bot
                    .send_message(ChatId(config.telegram_chat_id), &recovery_message)
                    .await
                {
                    log::error!("Failed to send Telegram message: {}", e);
                }
            }
            (Err(e), true) => {
                let error_message = format!(
                    "Server {} is down! Error: {}\nFailed {} consecutive times.",
                    url, e, config.failure_threshold
                );
                if let Err(e) = bot
                    .send_message(ChatId(config.telegram_chat_id), &error_message)
                    .await
                {
                    log::error!("Failed to send Telegram message: {}", e);
                }

                let mut state_guard = state.lock().await;
                if let Some(status) = state_guard.status.get_mut(&url) {
                    status.last_notification = Some(Utc::now());
                }
            }
            _ => {}
        }
    }
}

fn format_duration(secs: u64) -> String {
    format!(
        "{} days, {} hours, {} minutes, {} seconds",
        secs / 86400,
        (secs % 86400) / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

async fn handle_commands(
    bot: Bot,
    msg: Message,
    cmd: Command,
    state: Arc<Mutex<AppState>>,
    config: Arc<Config>,
) -> ResponseResult<()> {
    match cmd {
        Command::Uptime => {
            let state_guard = state.lock().await;
            let uptime_secs = state_guard.start_time.elapsed().as_secs();
            let uptime = format_duration(uptime_secs);

            let mut message = format!(
                "Monitoring statistics:\nUptime: {}\nMonitored servers:\n",
                uptime
            );

            for (url, server_stats) in &state_guard.servers {
                let availability = if server_stats.total_checks > 0 {
                    let success_rate = 100.0
                        * (server_stats.total_checks - server_stats.failed_checks) as f32
                        / server_stats.total_checks as f32;
                    format!("{:.2}%", success_rate)
                } else {
                    "no data".to_string()
                };

                let last_failure = server_stats
                    .last_failure
                    .map(|t| format!("{}", t.with_timezone(&Local)))
                    .unwrap_or_else(|| "never".to_string());

                message.push_str(&format!(
                    "{} - availability: {}, checks: {}, failures: {}, last failure: {}\n",
                    url,
                    availability,
                    server_stats.total_checks,
                    server_stats.failed_checks,
                    last_failure
                ));
            }

            bot.send_message(msg.chat.id, message).await?;
        }

        Command::Stats(server) => {
            let state_guard = state.lock().await;

            if let Some(server_stats) = state_guard.servers.get(&server) {
                let availability = if server_stats.total_checks > 0 {
                    let success_rate = 100.0
                        * (server_stats.total_checks - server_stats.failed_checks) as f32
                        / server_stats.total_checks as f32;
                    format!("{:.2}%", success_rate)
                } else {
                    "no data".to_string()
                };

                let last_failure = server_stats
                    .last_failure
                    .map(|t| format!("{}", t.with_timezone(&Local)))
                    .unwrap_or_else(|| "never".to_string());

                let consecutive_failures = state_guard
                    .status
                    .get(&server)
                    .map(|s| s.consecutive_failures)
                    .unwrap_or(0);

                let message = format!(
                    "Statistics for {}:\nAvailability: {}\nTotal checks: {}\nTotal failures: {}\nCurrent consecutive failures: {}\nLast failure: {}",
                    server,
                    availability,
                    server_stats.total_checks,
                    server_stats.failed_checks,
                    consecutive_failures,
                    last_failure
                );

                bot.send_message(msg.chat.id, message).await?;
            } else {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "Server '{}' not found in monitoring list. Available servers:\n{}",
                        server,
                        config.servers.join("\n")
                    ),
                )
                .await?;
            }
        }
    }

    Ok(())
}

fn load_config() -> Config {
    let config_file = fs::read_to_string("config.toml").expect("Failed to read config.toml");
    toml::from_str(&config_file).expect("Failed to parse config.toml")
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    log::info!("Starting iz-prober...");

    // Load configuration
    let config = Arc::new(load_config());
    log::info!(
        "Loaded configuration: {} servers to monitor",
        config.servers.len(),
    );

    let bot = Bot::new(&config.telegram_token);

    let state = Arc::new(Mutex::new(AppState {
        servers: HashMap::new(),
        status: HashMap::new(),
        start_time: Instant::now(),
    }));

    // Start monitoring tasks for each server
    for url in &config.servers {
        let bot_clone = bot.clone();
        let config_clone = config.clone();
        let state_clone = state.clone();
        let url = url.clone();

        tokio::spawn(
            async move { monitor_server(bot_clone, config_clone, state_clone, url).await },
        );
    }

    // Telegram command handler
    let handler = Update::filter_message().branch(
        dptree::entry()
            .filter_command::<Command>()
            .endpoint(handle_commands),
    );

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, config])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
