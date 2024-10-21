use crate::fsutil::{list_dir, read_capped};
use crate::{ReadError, Source};

/// The real filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct FsSource;

impl Source for FsSource {
    fn read(&self, path: &str) -> Result<Vec<u8>, ReadError> {
        read_capped(std::path::Path::new(path))
    }

    fn list(&self, dir: &str) -> Result<Vec<String>, ReadError> {
        list_dir(std::path::Path::new(dir))
    }
}
