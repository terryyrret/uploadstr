use poem::{http::StatusCode, Error, Result};
use std::path::Path;

#[cfg(test)]
use mockall::predicate::str;

#[cfg_attr(test, mockall::automock)]
trait FsOps {
    fn remove_file(&self, path: &dyn AsRef<Path>) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }
    fn create_dir_all(&self, path: &dyn AsRef<Path>) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }
    fn write(&self, path: &dyn AsRef<Path>, contents: &dyn AsRef<[u8]>) -> std::io::Result<()> {
        std::fs::write(path, contents)
    }
    fn read_dir(&self, path: &dyn AsRef<Path>) -> std::io::Result<std::fs::ReadDir> {
        std::fs::read_dir(path)
    }
}

struct RealFsOps {}

impl FsOps for RealFsOps {}

pub struct FS<'a> {
    fs: &'a dyn FsOps,
}

impl FS<'_> {
    pub fn new() -> Self {
        Self { fs: &RealFsOps {} }
    }

    pub fn delete_file(&self, files_dir: &str, filename: &str) -> Result<()> {
        let folder = Path::new(files_dir);
        let path = folder.join(filename);
        self.fs.remove_file(&path).map_err(|_| {
            Error::from_string(
                "Could not delete file from server...",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
    }

    pub fn save_file(&self, files_dir: &str, filename: &str, data: &[u8]) -> Result<()> {
        let folder = Path::new(files_dir);
        self.fs.create_dir_all(&folder).map_err(|_| {
            Error::from_string(
                "Could not save file to server.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })?;
        let path = folder.join(filename);
        self.fs.write(&path, &data).map_err(|_| {
            Error::from_string(
                "Could not save file to server.",
                StatusCode::INTERNAL_SERVER_ERROR,
            )
        })
    }

    pub fn get_files(&self, files_dir: &str) -> Result<Vec<String>> {
        self.fs
            .read_dir(&Path::new(files_dir))
            .map(|dir| {
                dir.flat_map(|res| res.map(|e| e.file_name().into_string()))
                    .flatten()
                    .collect::<Vec<String>>()
            })
            .map_err(|_| {
                Error::from_string(
                    "Failed to get list of files",
                    StatusCode::INTERNAL_SERVER_ERROR,
                )
            })
    }
}

#[cfg(test)]
mod test_delete_file {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn should_sucessfully_delete_file() {
        let mut mock = MockFsOps::new();
        mock.expect_remove_file()
            .withf(|p| p.as_ref() == Path::new("/Uploadstr/files/image.jpg"))
            .return_once(|_| Ok(()));

        let fs = FS { fs: &mock };

        let result = fs.delete_file("/Uploadstr/files", "image.jpg");

        assert!(result.is_ok());
    }

    #[test]
    fn should_return_err_from_remove_file_failing() {
        let mut mock = MockFsOps::new();
        mock.expect_remove_file()
            .withf(|p| p.as_ref() == Path::new("/Uploadstr/files/image.jpg"))
            .return_once(|_| Err(Error::from(ErrorKind::Other)));

        let fs = FS { fs: &mock };

        let result = fs.delete_file("/Uploadstr/files", "image.jpg");

        assert!(result.is_err());
    }
}

#[cfg(test)]
mod test_save_file {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn should_save_file_successfully() {
        let mut mock = MockFsOps::new();
        mock.expect_create_dir_all()
            .withf(|p| p.as_ref() == Path::new("/a/b/c/d/"))
            .return_once(|_| Ok(()));

        mock.expect_write()
            .withf(|p, d| p.as_ref() == Path::new("/a/b/c/d/f") && d.as_ref() == [1u8, 2u8, 3u8])
            .return_once(|_, _| Ok(()));

        let fs = FS { fs: &mock };

        let result = fs.save_file("/a/b/c/d/", "f", &[1u8, 2u8, 3u8]);

        result.unwrap();
    }

    #[test]
    fn should_return_err_from_create_dir_all_failing() {
        let mut mock = MockFsOps::new();
        mock.expect_create_dir_all()
            .withf(|p| p.as_ref() == Path::new("/a/b/c/d/"))
            .return_once(|_| Err(Error::from(ErrorKind::Other)));

        let fs = FS { fs: &mock };

        let result = fs.save_file("/a/b/c/d/", "image.jpg", &[1u8, 2u8, 3u8]);

        result.unwrap_err();
    }

    #[test]
    fn should_return_err_from_write_failing() {
        let mut mock = MockFsOps::new();
        mock.expect_create_dir_all()
            .withf(|p| p.as_ref() == Path::new("/a/b/c/d/"))
            .return_once(|_| Ok(()));

        mock.expect_write()
            .withf(|p, d| {
                p.as_ref() == Path::new("/a/b/c/d/img.png") && d.as_ref() == [1u8, 2u8, 3u8]
            })
            .return_once(|_, _| Err(Error::from(ErrorKind::Other)));

        let fs = FS { fs: &mock };

        let result = fs.save_file("/a/b/c/d/", "img.png", &[1u8, 2u8, 3u8]);

        result.unwrap_err();
    }
}
