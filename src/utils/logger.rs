use std::{io::Write, path::Path};



pub struct Logger {
    dir_path: String,
    level: LogLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub(crate) enum LogLevel {
    Trace = 0,
    Info = 1,
    Error = 2,
}

impl Logger {
    pub fn new(dir_path: &str, level: LogLevel) -> Self {
        Self {
            dir_path: dir_path.to_string(),
            level
        }
    }

    pub fn trace(&self, bytes: &[u8]) {
        if self.level <= LogLevel::Trace {
            self.log_bytes(bytes, LogLevel::Trace).unwrap()
        }
    }

    pub fn info(&self, bytes: &[u8]) {
        if self.level <= LogLevel::Info {
            self.log_bytes(bytes, LogLevel::Info).unwrap()
        }
    }

    pub fn error(&self, bytes: &[u8]) {
        if self.level <= LogLevel::Error {
            self.log_bytes(bytes, LogLevel::Error).unwrap()
        }
    }


    fn log_bytes(&self, bytes: &[u8], level: LogLevel) -> std::io::Result<()> {
        let path_str = format!("{}/{:?}/{}.log", self.dir_path, level, "binary");
        let path = Path::new(&path_str);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut f = std::fs::OpenOptions::new().create(true).append(true).open(path)?;
        f.write_all(bytes)?;
        f.write_all(b"\n")?;
        f.flush()?;
        Ok(())
    }
}