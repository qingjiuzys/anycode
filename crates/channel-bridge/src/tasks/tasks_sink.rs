//! Headless stdout sink for channel bridges and cron.

use std::io::Write;

/// Output target for headless task runs (stdio only).
pub enum ReplSink {
    Stdio,
}

impl ReplSink {
    pub fn line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        println!("{s}");
        let _ = std::io::stdout().flush();
    }

    pub fn eprint_line(&mut self, line: impl AsRef<str>) {
        let s = line.as_ref();
        eprintln!("{s}");
        let _ = std::io::stderr().flush();
    }

    pub fn push_stdout_str(&mut self, s: &str) {
        print!("{s}");
        let _ = std::io::stdout().flush();
    }
}
