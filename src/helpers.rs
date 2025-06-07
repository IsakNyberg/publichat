use std::io::{Read, Write};
use std::path::PathBuf;

pub type Res = Result<(), &'static str>;

pub fn full_write(stream: &mut impl Write, buf: &[u8], err: &'static str) -> Res {
    // writes buffer to stream and flushes it
    match stream.write(buf).and(stream.flush()) {
        Ok(_) => Ok(()),
        Err(_) => Err(err),
    }
}

pub fn read_exact(stream: &mut impl Read, buf: &mut [u8], err: &'static str) -> Res {
    stream.read_exact(buf).map_err(|_| err)
}

// owns all its data!
pub struct Globals {
    pub data_dir: PathBuf,
}
