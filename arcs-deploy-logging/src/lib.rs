mod r#macro; // Literal identifier syntax.
pub mod r#trait;
pub mod structs;

use lazy_init::Lazy;
use r#macro::logging_parts;
use lazy_static::lazy_static;

use r#trait::WriteImmut;
use structs::{PoisonErrorWrapper, ErrorWrapper};

use std::collections::HashMap;
use smallvec::{SmallVec, smallvec};

use log::{LevelFilter, Metadata, Record};

use std::fmt::{Display, Formatter};
use std::sync::{RwLock, Arc};
use chrono::{DateTime, Utc};


use std::io::{stderr, stdout, ErrorKind, Write};
use std::fs::{File, OpenOptions};
use std::path::Path;


pub (crate) use std::io::{Error as IOError, Result as IOResult};

#[doc(hidden)]
pub mod __internal_redirects {
    pub use log::{trace, debug, info, warn, error};
}
pub use log::Level;
pub use arcs_deploy_logging_proc_macro::with_target;
use const_format::concatcp;

macro_rules! color_text_fmt {
    (bold $color_sequence:literal, $formatting_lit:literal, $text:expr) => {
        format_args!(
            "{}{}{}",
            concatcp!("\x1b[1m", "\x1b[", $color_sequence, "m"),
            format_args!($formatting_lit, $text),
            "\x1b[0m",
        )
    };
    ($color_sequence:literal, $formatting_lit:literal, $text:expr) => {
        format_args!(
            "{}{}{}",
            concatcp!("\x1b[", $color_sequence, "m"),
            format_args!($formatting_lit, $text),
            "\x1b[0m",
        )
    };
    (option bold $color_sequence:literal, $formatting_lit:literal, $text:expr) => {
        {
            let module_path = $text;
            module_path.and(
                Some(format_args!(
                    "{}{}{}",
                    concatcp!("\x1b[1m", "\x1b[", $color_sequence, "m"),
                    format!($formatting_lit, $text.unwrap()),
                    "\x1b[0m",
                ))
            )
        }
    };
    (option $color_sequence:literal, $formatting_lit:literal, $text:expr) => {
        {
            let module_path = $text;
            module_path.and(
                Some(format_args!(
                    "{}{}{}",
                    concatcp!("\x1b[", $color_sequence, "m"),
                    format!($formatting_lit, $text.unwrap()),
                    "\x1b[0m",
                ))
            )
        }
    };
}


pub type LogLocationTargetMap<'a> = HashMap<Level, SmallVec<[LogLocationTarget<'a>; 6]>>;

#[derive(Debug)]
pub enum LogLocationTarget<'a> {
    StdOut,
    StdErr,
    File(&'a Path),
}

#[derive(Clone, Debug)]
enum WritableLogLocationTarget {
    StdOut,
    StdErr,
    File(Arc<RwLock<File>>),
}

impl WriteImmut for WritableLogLocationTarget {
    fn write(&self, buf: &[u8]) -> IOResult<usize> {
        
        match self {
            WritableLogLocationTarget::StdOut => Write::write(&mut stdout(), buf),
            WritableLogLocationTarget::StdErr => Write::write(&mut stderr(), buf),
            WritableLogLocationTarget::File(f) => f.write().map_or_else(
                |err| {
                    Err(IOError::new(
                        ErrorKind::Other,
                        PoisonErrorWrapper::from(err),
                    ))
                },
                |mut file| file.write(buf),
            ),
        }
    }
    fn write_vectored(&self, bufs: &[std::io::IoSlice<'_>]) -> IOResult<usize> {
        self.write(
            bufs.iter()
                .find(|buf| !buf.is_empty())
                .map_or(&[][..], |buf| &**buf),
        )
    }
    fn is_write_vectored(&self) -> bool {
        false
    }

    fn flush(&self) -> IOResult<()> {
        match self {
            WritableLogLocationTarget::StdOut => stdout().flush(),
            WritableLogLocationTarget::StdErr => stdout().flush(),
            WritableLogLocationTarget::File(f) => f.write().map_or_else(
                |err| {
                    Err(IOError::new(
                        ErrorKind::Other,
                        PoisonErrorWrapper::from(err),
                    ))
                },
                |mut file| file.flush(),
            ),
        }
    }
}

impl Write for WritableLogLocationTarget {
    fn write(&mut self, buf: &[u8]) -> IOResult<usize> {
        WriteImmut::write(self, buf)
    }
    fn flush(&mut self) -> IOResult<()> {
        WriteImmut::flush(self)
    }
}

#[derive(Debug, Default)]
pub struct WritableLogLocationTargetMap(HashMap<Level, SmallVec<[WritableLogLocationTarget; 6]>>);

impl Display for WritableLogLocationTargetMap {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:#?}", self))
    }
}

struct FileLogger {
    targets: RwLock<WritableLogLocationTargetMap>,
    name: Lazy<&'static str>,
    file_prefix: Lazy<(String, String)>,
}

struct LevelStringStruct {
    error: String,
    warn: String,
    info: String,
    debug: String,
    trace: String,
}

impl LevelStringStruct {
    fn get_level(&self, level: Level) -> &str {
        use Level::*;

        match level {
            Error => &self.error,
            Warn  => &self.warn,
            Info  => &self.info,
            Debug => &self.debug,
            Trace => &self.trace,
        }
    }
}

impl Default for LevelStringStruct {
    fn default() -> Self {
        Self {
            error: color_text_fmt!(bold "31", "{:<5}", "ERROR").to_string(),
            warn:  color_text_fmt!(bold "33", "{:<5}", "WARN ").to_string(),
            info:  color_text_fmt!(bold "36", "{:<5}", "INFO ").to_string(),
            debug: color_text_fmt!(bold "32", "{:<5}", "DEBUG").to_string(),
            trace: color_text_fmt!(bold "35", "{:<5}", "TRACE").to_string(),
        }
    }
}

lazy_static! {
    static ref LEVEL_STRINGS: LevelStringStruct = LevelStringStruct::default();
}

impl log::Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        Some(&metadata.target()) == self.name.get()
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) { return; }
        let target_map = match self.targets.read() {
            Ok(guard) => guard,
            Err(e) => {
                eprintln!("Logging target poisoned! {}", e);
                return;
            }
        };

        let utc: DateTime<Utc> = Utc::now();
        let (level, args) = (record.level(), record.args());

        for target in target_map.0.get(&record.level()).into_iter().flatten() {
            let stripped = with_path_prefix_stripped(record.file(), self.file_prefix.get());
            let full: Option<String> = stripped.map(|(prefix, body)| [prefix, body].into_iter().collect());

            let log_result = logging_parts!(
                target; <==
                "{} "   - Some(color_text_fmt!("38;5;147", "{}", utc.format("%b %d"))),
                "{}"    - Some(color_text_fmt!("38;5;86", "{}", utc.format("%H:%M:%S"))),
                "{} | " - Some(color_text_fmt!("38;5;23", "{}", utc.format("%.3f"))),

                "{} "   - color_text_fmt!(option "38;5;47", "{}", record.module_path()),
                
                "{}" - color_text_fmt!(option "38;5;159", "{}", full.as_ref())
                    => ":{}" - color_text_fmt!(option "38;5;159", "{:<3}", record.line())
                        => * "; ",
                "{} - " - Some(LEVEL_STRINGS.get_level(level)),
                "{}" - Some(args),
            );

            if let Err(error) = log_result {
                eprintln!("Failed to log to x! Error: {:?}", error);
            }
        }
    }

    fn flush(&self) {}
}

lazy_static! {
    static ref LOGGER: FileLogger = FileLogger {
        targets: RwLock::default(),
        name: Lazy::new(),
        file_prefix: Lazy::new(),
    };
}

pub fn with_path_prefix_stripped<'a>(path: Option<&'a str>, prefix: Option<&'a (String, String)>) -> Option<(&'a str, &'a str)> {
    if let (Some(path), Some((prefix, replace))) = (path, prefix) {
        path.strip_prefix(prefix).map_or_else(
            || Some(("", path)),
            |stripped| Some((replace, stripped)),
        )
    } else {
        path.map(|p| ("", p))
    }
}

pub fn set_up_logging(input: &LogLocationTargetMap, name: &'static str) -> IOResult<()> {
    LOGGER.name.get_or_create(|| name);
    if let Ok(value) = std::env::var("LOGGING_PREFIX_REPLACE") {
        if let Some((prefix, replace)) = value.split_once("->") {
            LOGGER.file_prefix.get_or_create(|| (prefix.to_string(), replace.to_string()));
        }
    }

    let mut target_hashmap = LOGGER
        .targets
        .write()
        .map_err(|error| IOError::new(ErrorKind::Other, PoisonErrorWrapper::from(error)))?;

    *target_hashmap = generate_writable_log_location_target_map(input, Utc::now());

    log::set_logger(&*LOGGER)
        .map(|()| log::set_max_level(LevelFilter::Trace))
        .map_err(|error| IOError::new(ErrorKind::Other, ErrorWrapper::from(error)))
}

pub fn generate_writable_log_location_target_map(
    from: &LogLocationTargetMap,
    time_startup: DateTime<Utc>,
) -> WritableLogLocationTargetMap {
    let mut file_map = HashMap::new();
    WritableLogLocationTargetMap(
        from.iter()
            .map(|(level, targets)| {
                (*level, {
                    targets
                        .iter()
                        .map(|target| match target {
                            LogLocationTarget::File(path) if !file_map.contains_key(path) => {
                                file_map.insert(
                                    path,
                                    Arc::new(RwLock::new({
                                        let mut file = OpenOptions::new().append(true).create(true).open(path)?;
                                        write!(file,
                                            "{:-^50}\n",
                                            format!(
                                                "Logging started at {}",
                                                time_startup.format("%b %d %H:%M:%S%.3f"),
                                            ),
                                        )?;
                                        file
                                    })),
                                );
                                Ok(WritableLogLocationTarget::File(
                                    file_map.get(path).ok_or(IOError::last_os_error())?.clone(),
                                ))
                            }
                            LogLocationTarget::File(path) => Ok(WritableLogLocationTarget::File(
                                file_map.get(path).ok_or(IOError::last_os_error())?.clone(),
                            )),
                            LogLocationTarget::StdOut => Ok(WritableLogLocationTarget::StdOut),
                            LogLocationTarget::StdErr => Ok(WritableLogLocationTarget::StdErr),
                        })
                        .inspect(|error| {
                            error
                                .as_ref()
                                .err()
                                .iter()
                                .for_each(|error: &&std::io::Error| eprintln!("{}", error))
                        })
                        .filter_map(Result::ok)
                        .collect()
                })
            })
            .collect(),
    )
}


lazy_static! {
    pub static ref ERR_FILE: &'static Path = Path::new("./err.log");
    pub static ref ERR_WARN_FILE: &'static Path = Path::new("./err_warn.log");
    pub static ref INFO_DEBUG_FILE: &'static Path = Path::new("./info_debug.log");
    pub static ref ALL_LOG_FILE: &'static Path = Path::new("./all.log");
    
    pub static ref DEFAULT_LOGGGING_TARGETS: LogLocationTargetMap<'static> = {
        use Level::*;
        use LogLocationTarget::*;
        vec![
            (Trace, smallvec![
                StdOut,
                File(&ALL_LOG_FILE),
            ]),
            (Debug, smallvec![
                // StdOut,
                File(&INFO_DEBUG_FILE),
                File(&ALL_LOG_FILE),
            ]),
            (Info, smallvec![
                StdOut,
                File(&INFO_DEBUG_FILE),
                File(&ALL_LOG_FILE),
            ]),
            (Warn, smallvec![
                StdErr,
                File(&ERR_WARN_FILE),
                File(&ALL_LOG_FILE),
            ]),
            (Error, smallvec![
                StdErr,
                File(&ERR_FILE),
                File(&ERR_WARN_FILE),
                File(&ALL_LOG_FILE),
            ]),

        ].into_iter().collect()
    };
}
