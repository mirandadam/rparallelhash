use anyhow::Result;
use std::f64;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};

const FKIB: f64 = (1024 * 1024) as f64;

pub struct OutputManager {
    writer: Box<dyn Write>,
    start_time: Instant,
    next_report: Instant,
    total_files: usize,
    processed_files: usize,
    total_bytes: u64,
    processed_bytes: u64,
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
            total_files: 0,
            processed_files: 0,
            total_bytes: 0,
            processed_bytes: 0,
        })
    }

    pub fn set_total_files(&mut self, total_files: usize) {
        self.total_files = total_files;
    }

    pub fn set_total_bytes(&mut self, total_bytes: u64) {
        self.total_bytes = total_bytes;
    }

    pub fn write_result(&mut self, result: &str) -> Result<()> {
        writeln!(self.writer, "{}", result)?;
        self.processed_files += 1;
        self.update_progress()?;
        Ok(())
    }

    pub fn update_bytes(&mut self, bytes: u64) -> Result<()> {
        self.processed_bytes += bytes;
        self.update_progress()?;
        Ok(())
    }

    fn update_progress(&mut self) -> Result<()> {
        let elapsed = self.start_time.elapsed();
        let speed = self.processed_bytes as f64 / elapsed.as_secs_f64() / FKIB;
        let progress = if self.total_files > 0 {
            self.processed_files as f64 / self.total_files as f64 * 100.0
        } else {
            0.0
        };

        if Instant::now() > self.next_report {
            eprint!(
                "\rProgress: {:.2}% ({}/{} files, {:.2} MiB/s)        ",
                progress, self.processed_files, self.total_files, speed
            );
            io::stderr().flush()?;
            self.next_report += Duration::from_millis(200);
        }
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        let elapsed = self.start_time.elapsed();
        let speed = self.processed_bytes as f64 / elapsed.as_secs_f64() / FKIB;
        let formatted_bytes = format_bytes(self.processed_bytes);
        eprintln!(
            "\nFinished: {} files processed, {:.2} MB/s, total time: {}, total bytes: {}",
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
