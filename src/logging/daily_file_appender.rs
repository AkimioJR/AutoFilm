use chrono::{Datelike, NaiveDate, Utc};
use chrono_tz::Tz;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

pub(super) struct DailyFileAppender {
    directory: PathBuf,
    timezone: Tz,
    current_date: Option<NaiveDate>,
    file: Option<File>,
}

impl DailyFileAppender {
    pub(super) fn new(directory: PathBuf, timezone: Tz) -> io::Result<Self> {
        std::fs::create_dir_all(&directory)?;

        Ok(Self {
            directory,
            timezone,
            current_date: None,
            file: None,
        })
    }

    fn current_date(&self) -> NaiveDate {
        Utc::now().with_timezone(&self.timezone).date_naive()
    }

    fn ensure_file(&mut self) -> io::Result<&mut File> {
        let date = self.current_date();

        if self.current_date != Some(date) {
            let path = log_file_path(&self.directory, date);
            self.file = Some(OpenOptions::new().create(true).append(true).open(path)?);
            self.current_date = Some(date);
        }

        self.file
            .as_mut()
            .ok_or_else(|| io::Error::other("daily log file was not opened"))
    }
}

impl Write for DailyFileAppender {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.ensure_file()?.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

fn log_file_path(directory: &Path, date: NaiveDate) -> PathBuf {
    directory.join(format!(
        "{:04}-{:02}-{:02}.log",
        date.year(),
        date.month(),
        date.day()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn daily_log_file_path_uses_date_suffix() {
        let path = log_file_path(
            Path::new("/tmp/autofilm-logs"),
            NaiveDate::from_ymd_opt(2026, 6, 5).unwrap(),
        );

        assert_eq!(path, PathBuf::from("/tmp/autofilm-logs/2026-06-05.log"));
    }
}
