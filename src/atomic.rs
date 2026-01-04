use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use fs2::FileExt;
use tempfile::NamedTempFile;

struct FileLock {
    _file: File,
}

impl FileLock {
    fn lock(path: &Path) -> std::io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;

        // Blocks until exclusive lock is acquired
        file.lock_exclusive()?;

        Ok(Self { _file: file })
    }
}

pub struct AtomicFile {
    path: PathBuf,
}

impl AtomicFile {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn read(&self) -> std::io::Result<String> {
        let file = OpenOptions::new()
            .read(true)
            .create(true)
            .open(&self.path)?;

        file.lock_shared()?;

        let mut buf = String::new();
        (&file).read_to_string(&mut buf)?;

        Ok(buf)
    }

    pub fn write(&self, contents: &str) -> std::io::Result<()> {
        let _lock = FileLock::lock(&self.path)?;

        let dir = self.path.parent().unwrap_or(Path::new("."));
        let mut tmp = NamedTempFile::new_in(dir)?;

        tmp.write_all(contents.as_bytes())?;
        tmp.flush()?;
        tmp.as_file().sync_all()?;

        tmp.persist(&self.path)?;

        Ok(())
    }
}
