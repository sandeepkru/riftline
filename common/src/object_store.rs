use std::path::PathBuf;
use tokio::fs;
use tokio::io;
use tonic::async_trait;

/// Trait representing a simple object store.
#[async_trait]
pub trait ObjectStore {
    /// Store an object at the given key.
    async fn put(&self, key: &str, bytes: &[u8]) -> io::Result<()>;

    /// Retrieve the object stored at the given key.
    async fn get(&self, key: &str) -> io::Result<Vec<u8>>;
}

/// Local filesystem implementation of [`ObjectStore`].
pub struct LocalFileSystemStore {
    root: PathBuf,
}

impl LocalFileSystemStore {
    /// Create a new store rooted at the provided directory.
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    fn path_for_key(&self, key: &str) -> PathBuf {
        self.root.join(key)
    }
}

#[async_trait]
impl ObjectStore for LocalFileSystemStore {
    async fn put(&self, key: &str, bytes: &[u8]) -> io::Result<()> {
        let path = self.path_for_key(key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }
        fs::write(path, bytes).await
    }

    async fn get(&self, key: &str) -> io::Result<Vec<u8>> {
        let path = self.path_for_key(key);
        fs::read(path).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn write_and_read_object() {
        let dir = tempdir().unwrap();
        let store = LocalFileSystemStore::new(dir.path());
        store.put("foo", b"bar").await.unwrap();
        let data = store.get("foo").await.unwrap();
        assert_eq!(data, b"bar");
    }

    #[tokio::test]
    async fn creates_intermediate_directories() {
        let dir = tempdir().unwrap();
        let store = LocalFileSystemStore::new(dir.path());
        store.put("nested/path/file", b"baz").await.unwrap();
        let data = store.get("nested/path/file").await.unwrap();
        assert_eq!(data, b"baz");
    }
}
