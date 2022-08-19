use super::{IOError, IOResult};
use std::io::ErrorKind;

pub trait WriteImmut {
    fn write(&self, buf: &[u8]) -> IOResult<usize>;
    fn flush(&self) -> IOResult<()>;
    fn write_fmt(&self, fmt: std::fmt::Arguments<'_>) -> IOResult<()> {
        self.write_all(format!("{}", fmt).as_bytes())
    }
    fn write_all(&self, mut buf: &[u8]) -> IOResult<()> {
        while !buf.is_empty() {
            match self.write(buf) {
                Ok(0) => {
                    return Err(IOError::new(
                        ErrorKind::WriteZero,
                        "failed to write whole buffer",
                    ));
                }
                Ok(n) => buf = &buf[n..],
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
    fn write_vectored(&self, bufs: &[std::io::IoSlice<'_>]) -> IOResult<usize>;
    fn is_write_vectored(&self) -> bool;
}