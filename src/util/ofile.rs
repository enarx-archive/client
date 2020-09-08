use std::fs::File;
use std::io::{prelude::*, IoSlice, IoSliceMut, Result, Seek, SeekFrom};
use std::path::Path;

#[derive(Debug)]
struct Inner<T: AsRef<Path>> {
    file: File,
    path: T,
}

/// An output file
///
/// This is a trivial wrapper for the `File` type which removes the file when
/// dropped unless `Self::done()` is called. This means that if an error
/// occurs during the production of output, the file is cleaned up
/// automatically.
#[derive(Debug)]
pub struct OutputFile<T: AsRef<Path>>(Option<Inner<T>>);

impl<T: AsRef<Path>> OutputFile<T> {
    #[inline]
    pub fn create(path: T) -> std::io::Result<Self> {
        Ok(Self(Some(Inner {
            file: File::create(path.as_ref())?,
            path,
        })))
    }

    #[inline]
    pub fn done(mut self) -> File {
        self.0.take().unwrap().file
    }
}

impl<T: AsRef<Path>> Drop for OutputFile<T> {
    fn drop(&mut self) {
        if let Some(Inner { file, path }) = self.0.take() {
            drop(file);
            drop(std::fs::remove_file(path));
        }
    }
}

#[cfg(not(target_os = "windows"))]
impl<T: AsRef<Path>> std::os::unix::io::AsRawFd for OutputFile<T> {
    #[inline]
    fn as_raw_fd(&self) -> std::os::unix::io::RawFd {
        self.0.as_ref().unwrap().file.as_raw_fd()
    }
}

#[cfg(target_os = "windows")]
impl<T: AsRef<Path>> std::os::windows::io::AsRawHandle for OutputFile<T> {
    #[inline]
    fn as_raw_handle(&self) -> RawHandle {
        self.0.as_ref().unwrap().file.as_raw_handle()
    }
}

#[cfg(not(target_os = "windows"))]
impl<T: AsRef<Path>> std::os::unix::fs::FileExt for OutputFile<T> {
    #[inline]
    fn read_at(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        self.0.as_ref().unwrap().file.read_at(buf, offset)
    }

    #[inline]
    fn write_at(&self, buf: &[u8], offset: u64) -> Result<usize> {
        self.0.as_ref().unwrap().file.write_at(buf, offset)
    }
}

#[cfg(target_os = "windows")]
impl<T: AsRef<Path>> std::os::windows::fs::FileExt for OutputFile<T> {
    #[inline]
    fn seek_read(&self, buf: &mut [u8], offset: u64) -> Result<usize> {
        self.0.as_ref().unwrap().file.seek_read(buf, offset)
    }

    #[inline]
    fn seek_write(&self, buf: &[u8], offset: u64) -> Result<usize> {
        self.0.as_ref().unwrap().file.seek_write(buf, offset)
    }
}

impl<T: AsRef<Path>> Read for OutputFile<T> {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        self.0.as_mut().unwrap().file.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [IoSliceMut<'_>]) -> Result<usize> {
        self.0.as_mut().unwrap().file.read_vectored(bufs)
    }
}

impl<T: AsRef<Path>> Write for OutputFile<T> {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.0.as_mut().unwrap().file.write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.0.as_mut().unwrap().file.write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> Result<()> {
        self.0.as_mut().unwrap().file.flush()
    }
}

impl<T: AsRef<Path>> Seek for OutputFile<T> {
    #[inline]
    fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
        self.0.as_mut().unwrap().file.seek(pos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed() {
        let pid = std::process::id();
        let name = format!("/tmp/output-file.{}", pid);
        let path = Path::new(&name);

        let file = OutputFile::create(path).unwrap();
        assert!(path.exists());
        assert!(path.is_file());
        drop(file);
        assert!(!path.exists());
    }

    #[test]
    fn retained() {
        let pid = std::process::id();
        let name = format!("/tmp/output-file.{}", pid);
        let path = Path::new(&name);

        let file = OutputFile::create(path).unwrap();
        assert!(path.exists());
        assert!(path.is_file());
        drop(file.done());
        assert!(path.exists());

        std::fs::remove_file(path).unwrap();
    }
}
