#![allow(dead_code)]
use anyhow::Result;
use once_cell::sync::Lazy;
use std::{
    fmt,
    fs::{File, OpenOptions},
    io::Write,
    path::Path,
    sync::Mutex,
};

const DEFAULT_FILENAME: &str = "./.yash.log";

#[derive(Debug, Clone, Copy)]
enum LogLevel {
    Error,
    Warn,
    Info,
    Todo,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Error => write!(f, "[ERROR]"),
            Self::Warn => write!(f, "[WARN]"),
            Self::Info => write!(f, "[INFO]"),
            Self::Todo => write!(f, "[TODO]"),
        }
    }
}

static LOGGER: Lazy<Mutex<Logger>> = Lazy::new(|| Mutex::new(Logger::default()));

struct Logger {
    file: Option<File>,
}

impl Logger {
    fn new() -> Self {
        Self { file: None }
    }

    fn init(&mut self, filename: impl AsRef<Path>) -> Result<&mut Self> {
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(filename)?;
        self.file = Some(file);
        Ok(self)
    }

    fn log_content(&self, log_level: LogLevel, message: &str) -> Result<()> {
        let mut file = self.file.as_ref().unwrap();
        writeln!(file, "{} {}", log_level, message)?;
        file.flush()?;
        Ok(())
    }

    fn info(&self, message: &str) -> Result<()> {
        self.log_content(LogLevel::Info, message)?;
        Ok(())
    }

    fn warn(&self, message: &str) -> Result<()> {
        self.log_content(LogLevel::Warn, message)?;
        Ok(())
    }

    fn error(&self, message: &str) -> Result<()> {
        self.log_content(LogLevel::Error, message)?;
        Ok(())
    }

    fn todo(&self, message: &str) -> Result<()> {
        self.log_content(LogLevel::Todo, message)?;
        Ok(())
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        let mut instance = Self::new();
        if let Some(ref file) = self.file {
            instance.file = file.try_clone().ok();
        }
        instance
    }
}

impl Default for Logger {
    fn default() -> Self {
        let mut instance = Self::new();
        let instance = instance.init(DEFAULT_FILENAME).unwrap();
        instance.clone()
    }
}

impl Drop for Logger {
    fn drop(&mut self) {
        if let Some(mut file) = self.file.take() {
            let _ = file.flush().unwrap();
        }
    }
}

pub fn init_loggery(filename: impl AsRef<Path>) -> Result<()> {
    let _ = LOGGER.lock().unwrap().init(filename)?;
    Ok(())
}

#[macro_export]
macro_rules! infoy {
    ($( $arg:tt )*) => {{
        let logger = LOGGER.lock().unwrap();
        logger.info(&format!("{}", format_args!($($arg)*)))
    }};
}

#[macro_export]
macro_rules! warny {
    ($( $arg:tt )*) => {{
        let logger = LOGGER.lock().unwrap();
        logger.warn(&format!("{}", format_args!($($arg)*)))
    }};
}

#[macro_export]
macro_rules! errory {
    ($( $arg:tt )*) => {{
        let logger = LOGGER.lock().unwrap();
        logger.error(&format!("{}", format_args!($($arg)*)))
    }};
}

#[macro_export]
macro_rules! todoy {
    ($( $arg:tt )*) => {{
        let logger = LOGGER.lock().unwrap();
        let _ = logger.todo(&format!("{}", format_args!($($arg)*)));
        todo!($($arg)*);
    }};
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;

    use super::*;
    use std::{env, fs, path::PathBuf};

    const CARGO_TOML: &str = "Cargo.toml";

    fn find_project_dir() -> Option<PathBuf> {
        let mut current_dir = env::current_dir().ok()?;
        while !current_dir.join(CARGO_TOML).is_file() {
            if !current_dir.pop() {
                return None;
            }
        }
        Some(current_dir)
    }

    fn clear_file(file_path: impl AsRef<Path>) -> Result<()> {
        let mut file = fs::File::create(file_path)?;
        file.set_len(0)?;
        file.flush()?;
        Ok(())
    }

    fn setup(should_clear: bool) -> Result<PathBuf> {
        let mut project = find_project_dir().unwrap_or(PathBuf::new());
        project.push(".yash.log");
        let _ = init_loggery(project.clone());
        if should_clear {
            clear_file(project.clone())?;
        }
        Ok(project)
    }

    fn test_infoy_1() -> Result<bool> {
        let project = setup(true)?;

        infoy!("yash is testing {} {} {}!!!", 1, 2, 3)?;

        let content = fs::read_to_string(project).expect("file should not be empty");
        let expected = "[INFO] yash is testing 1 2 3!!!\n";
        assert_eq!(content, expected);
        Ok(content == expected)
    }

    fn test_infoy_2() -> Result<bool> {
        //# NOT CLEARING FILE FOR INEQUALITY ASSERTION
        let project = setup(false)?;

        infoy!("yash is testing {} {} {}!!!", 1, 2, 3)?;

        let content = fs::read_to_string(project).expect("file should not be empty");
        let expected = "[INFO] yash is testing 1 2 3!!!\n";
        assert_ne!(content, expected);
        Ok(content != expected)
    }

    fn test_warny() -> Result<bool> {
        let project = setup(true)?;

        warny!("yash is testing {} {} {}!!!", 1, 2, 3)?;

        let content = fs::read_to_string(project).expect("file should not be empty");
        let expected = "[WARN] yash is testing 1 2 3!!!\n";
        assert_eq!(content, expected);
        Ok(content == expected)
    }

    fn test_errory() -> Result<bool> {
        let project = setup(true)?;

        errory!("yash is testing {} {} {}!!!", 1, 2, 3)?;

        let content = fs::read_to_string(project).expect("file should not be empty");
        let expected = "[ERROR] yash is testing 1 2 3!!!\n";
        assert_eq!(content, expected);
        Ok(content == expected)
    }

    #[test]
    fn test_all() {
        let a = test_infoy_1().unwrap();
        let b = test_infoy_2().unwrap();
        let c = test_warny().unwrap();
        let d = test_errory().unwrap();
        assert_eq!(a && b && c && d, true);
    }

    #[test]
    #[should_panic]
    fn test_todoy() {
        let mut num = 0;
        let mut msgs = String::new();
        loop {
            let value = match num {
                9 => todoy!("99 or not, its still a value | seen so far: {}", msgs),
                x => x,
            };
            msgs.push_str(&value.to_string());
            msgs.push(' ');
            num += 1;
        }
    }
}
