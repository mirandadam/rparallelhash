use anyhow::Result;
use std::collections::VecDeque;
use std::f64;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const FKIB: f64 = (1024 * 1024) as f64;
const UPDATE_INTERVAL: Duration = Duration::from_millis(200);
const THROUGHPUT_WINDOW: Duration = Duration::from_secs(1);

pub struct OutputManager {
    writer: Box<dyn Write>,
    start_time: Instant,
    next_report: Instant,
    processed_files: usize,
    processed_bytes: u64,
    recent_updates: VecDeque<(Instant, u64)>,
}

impl OutputManager {
    pub fn new(output_path: Option<&Path>) -> Result<Self> {
        let writer: Box<dyn Write> = if let Some(path) = output_path {
            Box::new(File::create(path)?)
        } else {
            Box::new(io::stdout())
        };

        Ok(Self {
            writer,
            start_time: Instant::now(),
            next_report: Instant::now(),
            processed_files: 0,
            processed_bytes: 0,
            recent_updates: VecDeque::new(),
        })
    }

    pub fn write_result(&mut self, result: &str) -> Result<()> {
        writeln!(self.writer, "{}", result)?;
        self.processed_files += 1;
        self.update_progress()?;
        Ok(())
    }

    pub fn update_bytes(&mut self, bytes: u64) -> Result<()> {
        self.processed_bytes += bytes;
        let now = Instant::now();

        self.recent_updates.push_back((now, self.processed_bytes));

        // Remove outdated entries
        while let Some((time, _)) = self.recent_updates.front() {
            if now.duration_since(*time) > THROUGHPUT_WINDOW {
                self.recent_updates.pop_front();
            } else {
                break;
            }
        }

        self.update_progress()?;
        Ok(())
    }

    fn calculate_current_throughput(&self) -> f64 {
        if self.recent_updates.len() < 2 {
            return 0.0;
        }
        let (start_time, start_bytes) = self.recent_updates.front().unwrap();
        let (end_time, end_bytes) = self.recent_updates.back().unwrap();
        let duration = end_time.duration_since(*start_time).as_secs_f64();
        if duration > 0.0 {
            let bytes_processed = end_bytes - start_bytes;
            bytes_processed as f64 / duration / FKIB
        } else {
            0.0
        }
    }

    fn update_progress(&mut self) -> Result<()> {
        let now = Instant::now();
        if now >= self.next_report {
            let elapsed = self.start_time.elapsed();
            let avg_speed = self.processed_bytes as f64 / elapsed.as_secs_f64() / FKIB;
            let current_speed = self.calculate_current_throughput();
            eprint!(
                "\rProcessed: {} files, {}, Avg: {:.2} MiB/s, Current: {:.2} MiB/s        ",
                self.processed_files,
                format_bytes(self.processed_bytes),
                avg_speed,
                current_speed
            );
            io::stderr().flush()?;
            self.next_report = now + UPDATE_INTERVAL;
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        let elapsed = self.start_time.elapsed();
        let speed = self.processed_bytes as f64 / elapsed.as_secs_f64() / FKIB;
        let formatted_bytes = format_bytes(self.processed_bytes);
        eprintln!(
            "\nFinished: {} files processed, {:.2} MiB/s, total time: {}, total bytes: {}",
            self.processed_files,
            speed,
            format_duration(elapsed),
            formatted_bytes
        );
        Ok(())
    }
}

fn format_bytes(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = KIB * 1024;
    const GIB: u64 = MIB * 1024;
    const TIB: u64 = GIB * 1024;

    if bytes >= TIB {
        format!("{:.2} TiB", bytes as f64 / TIB as f64)
    } else if bytes >= GIB {
        format!("{:.2} GiB", bytes as f64 / GIB as f64)
    } else if bytes >= MIB {
        format!("{:.2} MiB", bytes as f64 / MIB as f64)
    } else if bytes >= KIB {
        format!("{:.2} KiB", bytes as f64 / KIB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

fn format_duration(duration: Duration) -> String {
    let total_seconds = duration.as_secs();
    let hours = total_seconds / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
