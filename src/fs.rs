use std::fs;
use std::io;
use std::io::Write;
use std::path;

use globwalk;
use tempfile;

use errors;
use errors::ResultChainExt;

/// A directory in the filesystem that is automatically deleted when it goes out of scope.
pub use tempfile::TempDir;

/// Extend `TempDir` to perform operations on relative paths within the temp directory via
/// `ChildPath`.
pub trait TempDirChildExt {
    /// Create a path within the temp directory.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_fs::prelude::*;
    ///
    /// let temp = assert_fs::TempDir::new().unwrap();
    /// println!("{:?}", temp.path());
    /// println!("{:?}", temp.child("foo/bar.txt").path());
    /// temp.close().unwrap();
    /// ```
    fn child<P>(&self, path: P) -> ChildPath
    where
        P: AsRef<path::Path>;
}

impl TempDirChildExt for tempfile::TempDir {
    fn child<P>(&self, path: P) -> ChildPath
    where
        P: AsRef<path::Path>,
    {
        ChildPath::new(self.path().join(path.as_ref()))
    }
}

/// A path within a `TempDir`
pub struct ChildPath {
    path: path::PathBuf,
}

impl ChildPath {
    /// Wrap a path for use with special built extension traits.
    ///
    /// See trait implementations or `TempDirChildExt` for more details.
    pub fn new<P>(path: P) -> Self
    where
        P: Into<path::PathBuf>,
    {
        Self { path: path.into() }
    }

    /// Access the path.
    pub fn path(&self) -> &path::Path {
        &self.path
    }
}

/// Extend `ChildPath` to create empty files.
pub trait ChildPathTouchExt {
    /// Create an empty file at `ChildPath`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_fs::prelude::*;
    ///
    /// let temp = assert_fs::TempDir::new().unwrap();
    /// temp.child("foo.txt").touch().unwrap();
    /// temp.close().unwrap();
    /// ```
    fn touch(&self) -> io::Result<()>;
}

impl ChildPathTouchExt for ChildPath {
    fn touch(&self) -> io::Result<()> {
        touch(self.path())
    }
}

/// Extend `ChildPath` to write binary files.
pub trait ChildPathWriteBinExt {
    /// Write a binary file at `ChildPath`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_fs::prelude::*;
    ///
    /// let temp = assert_fs::TempDir::new().unwrap();
    /// temp.child("foo.txt").write_binary(b"To be or not to be...").unwrap();
    /// temp.close().unwrap();
    /// ```
    fn write_binary(&self, data: &[u8]) -> io::Result<()>;
}

impl ChildPathWriteBinExt for ChildPath {
    fn write_binary(&self, data: &[u8]) -> io::Result<()> {
        write_binary(self.path(), data)
    }
}

/// Extend `ChildPath` to write text files.
pub trait ChildPathWriteStrExt {
    /// Write a text file at `ChildPath`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_fs::prelude::*;
    ///
    /// let temp = assert_fs::TempDir::new().unwrap();
    /// temp.child("foo.txt").write_str("To be or not to be...").unwrap();
    /// temp.close().unwrap();
    /// ```
    fn write_str(&self, data: &str) -> io::Result<()>;
}

impl ChildPathWriteStrExt for ChildPath {
    fn write_str(&self, data: &str) -> io::Result<()> {
        write_str(self.path(), data)
    }
}

/// Extend `TempDir` to copy files into it.
pub trait TempDirCopyExt {
    /// Copy files and directories into the current path from the `source` according to the glob
    /// `patterns`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use assert_fs::prelude::*;
    ///
    /// let temp = assert_fs::TempDir::new().unwrap();
    /// temp.copy_from(".", &["*.rs"]).unwrap();
    /// temp.close().unwrap();
    /// ```
    fn copy_from<P, S>(&self, source: P, patterns: &[S]) -> Result<(), errors::FixtureError>
    where
        P: AsRef<path::Path>,
        S: AsRef<str>;
}

impl TempDirCopyExt for tempfile::TempDir {
    fn copy_from<P, S>(&self, source: P, patterns: &[S]) -> Result<(), errors::FixtureError>
    where
        P: AsRef<path::Path>,
        S: AsRef<str>,
    {
        copy_from(self.path(), source.as_ref(), patterns)
    }
}

impl TempDirCopyExt for ChildPath {
    fn copy_from<P, S>(&self, source: P, patterns: &[S]) -> Result<(), errors::FixtureError>
    where
        P: AsRef<path::Path>,
        S: AsRef<str>,
    {
        copy_from(self.path(), source.as_ref(), patterns)
    }
}

fn touch(path: &path::Path) -> io::Result<()> {
    fs::File::create(path)?;
    Ok(())
}

fn write_binary(path: &path::Path, data: &[u8]) -> io::Result<()> {
    let mut file = fs::File::create(path)?;
    file.write_all(data)?;
    Ok(())
}

fn write_str(path: &path::Path, data: &str) -> io::Result<()> {
    write_binary(path, data.as_bytes())
}

fn copy_from<S>(
    target: &path::Path,
    source: &path::Path,
    patterns: &[S],
) -> Result<(), errors::FixtureError>
where
    S: AsRef<str>,
{
    for entry in globwalk::GlobWalker::from_patterns(source, patterns)
        .chain(errors::FixtureError::default())?
        .follow_links(true)
    {
        let entry = entry.chain(errors::FixtureError::default())?;
        let rel = entry
            .path()
            .strip_prefix(source)
            .expect("entries to be under `source`");
        let target_path = target.join(rel);
        if entry.file_type().is_dir() {
            fs::create_dir_all(target_path).chain(errors::FixtureError::default())?;
        } else if entry.file_type().is_file() {
            fs::create_dir_all(target_path.parent().expect("at least `target` exists"))
                .chain(errors::FixtureError::default())?;
            fs::copy(entry.path(), target_path).chain(errors::FixtureError::default())?;
        }
    }
    Ok(())
}
