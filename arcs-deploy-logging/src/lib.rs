mod r#macro; // Literal identifier syntax.
pub mod r#trait;

use lazy_init::Lazy;
use r#macro::logging_parts;
use lazy_static::lazy_static;

use r#trait::WriteImmut;

use arcs_deploy_shared_structs::*;
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
            let log_result = logging_parts!(
                target; <==
                "{} | " - Some(utc.format("%b %d %H:%M:%S%.3f")),
                "{}; " - record.module_path(),
                "{}" - record.file()
                    => ":{}" - record.line()
                        => * "; ",
                "{} - " - Some(level),
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
    };
}

pub fn set_up_logging(input: &LogLocationTargetMap, name: &'static str) -> IOResult<()> {
    LOGGER.name.get_or_create(|| name);

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

    
    pub static ref DEFAULT_LOGGGING_TARGETS: LogLocationTargetMap<'static> = {
        use Level::*;
        use LogLocationTarget::*;
        vec![
            (Trace, smallvec![
                StdOut,
            ]),
            (Debug, smallvec![
                StdOut,
                File(&INFO_DEBUG_FILE),
            ]),
            (Info, smallvec![
                StdOut,
                File(&INFO_DEBUG_FILE),
            ]),
            (Warn, smallvec![
                StdErr,
                File(&ERR_WARN_FILE),
            ]),
            (Error, smallvec![
                StdErr,
                File(&ERR_FILE),
                File(&ERR_WARN_FILE),
            ]),
            
        ].into_iter().collect()
    };
}
