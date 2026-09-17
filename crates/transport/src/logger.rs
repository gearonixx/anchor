use std::io::{IsTerminal, Write};

use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record};


// TODO: why do we need this?

struct Logger {
    pretty: bool,
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let mut out = std::io::stderr().lock();
        if self.pretty {
            let _ = writeln!(
                out,
                "{} {:<5} [{}] {}",
                Local::now().format("%H:%M:%S%.3f"),
                record.level(),
                record.target(),
                record.args()
            );
        } else {
            let _ = writeln!(
                out,
                "{{\"time\":\"{}\",\"level\":\"{}\",\"target\":\"{}\",\"msg\":\"{}\"}}",
                Local::now().to_rfc3339(),
                level_name(record.level()),
                escape(record.target()),
                escape(&record.args().to_string())
            );
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Error => "error",
        Level::Warn => "warn",
        Level::Info => "info",
        Level::Debug => "debug",
        Level::Trace => "trace",
    }
}

fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());

    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }

    out
}

pub fn init() {
    // so every log::debug! in the crate is dead at runtime.
    log::set_max_level(LevelFilter::Info);
    let pretty = std::io::stderr().is_terminal();
    let _ = log::set_boxed_logger(Box::new(Logger { pretty }));
}
