use std::io::Write;

use env_logger::{Builder, Env};
use log::{Level, LevelFilter};

pub fn setup_logger() {
    let env_level = Env::default().default_filter_or("info");
    let env_level_str = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    let env_levelfilter = env_level_str.parse().unwrap_or_else(|_| LevelFilter::Info);

    Builder::from_env(env_level)
        .filter_level(LevelFilter::Off)
        .filter_module("sonorust", env_levelfilter)
        .filter_module("sonorust_db", env_levelfilter)
        .filter_module("sonorust_logger", env_levelfilter)
        .filter_module("engtokana", env_levelfilter)
        .filter_module("setting_inputter", env_levelfilter)
        .format(move |buf, record| {
            let level = record.level();
            let level_color = match level {
                Level::Error => "\x1B[31m", // 赤
                Level::Warn => "\x1B[33m",  // 黄
                Level::Info => "\x1B[32m",  // 緑
                Level::Debug => "\x1B[34m", // 青
                Level::Trace => "\x1B[35m", // マゼンタ
            };

            let space_len = 5 - record.level().as_str().len();
            let level_name = record.level().to_string() + &" ".repeat(space_len);

            let reset = "\x1B[0m";
            let green = "\x1B[32m";

            // 表示レベルが Debug かどうかで動作を変える
            if env_levelfilter == LevelFilter::Debug {
                buf.write_fmt(format_args!(
                    "{level_color}{}{reset} | {green}{}{reset} | {} - {}:{}\n",
                    level_name,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.args(),
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                ))
            } else {
                buf.write_fmt(format_args!(
                    "{level_color}{}{reset} | {green}{}{reset} | {}\n",
                    level_name,
                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                    record.args()
                ))
            }
        })
        .init();
}

#[cfg(test)]
mod tests {

    #[ignore]
    #[test]
    fn log_test() {
        std::env::set_var("RUST_LOG", "trace");
        super::setup_logger();

        log::trace!("trace_log");
        log::debug!("debug_log");
        log::info!("info_log");
        log::warn!("warn_log");
        log::error!("error_log");
    }

    #[ignore]
    #[test]
    fn log_test_debug() {
        std::env::set_var("RUST_LOG", "debug");
        super::setup_logger();

        log::trace!("trace_log");
        log::debug!("debug_log");
        log::info!("info_log");
        log::warn!("warn_log");
        log::error!("error_log");
    }
}
