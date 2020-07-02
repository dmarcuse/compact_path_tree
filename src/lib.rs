use std::fs::DirEntry;
use std::io;
use std::path::{Component, Components, Path, PathBuf};

/// A visitor that can determine which paths should be included and how errors
/// are handled, or just view paths as the tree is constructed.
pub trait PathVisitor {
    /// Determine whether the given entry should be included in the tree.
    ///
    /// When `Ok(true)` is returned, the entry is included. When `Ok(false)` is
    /// returned, the entry is omitted, including any children for directories.
    /// When `Err(..)` is returned, the item is omitted and
    /// `PathVisitor::handle_error` is used to determine whether the operation
    /// should fail or not.
    fn filter(&mut self, _entry: &DirEntry) -> io::Result<bool> {
        Ok(true)
    }

    /// A general-purpose function for any logic involving included entries.
    /// This is called after `filter` and only for entries for which `filter`
    /// returned `Ok(true)`.
    fn visit(&mut self, _entry: &DirEntry) -> io::Result<()> {
        Ok(())
    }

    /// Handle a given IO error, determining whether it should be fatal or
    /// not.
    ///
    /// `Some(..)` indicates that the error is fatal and should stop traversal,
    /// and `None` indicates that the error is non-fatal and should be ignored.
    /// When an error is ignored, the
    ///
    /// This function will be called for any error that occurs, including errors
    /// from `PathVisitor::filter` or `PathVisitor::visit`.
    ///
    /// The default implementation logs `PermissionDenied` errors to stderr but
    /// doesn't stop.
    fn handle_error(
        &mut self,
        error: io::Error,
        directory: &Path,
        entry: Option<&DirEntry>,
    ) -> Option<io::Error> {
        match error.kind() {
            io::ErrorKind::PermissionDenied => {
                let description = match entry {
                    None => format!("item in `{}`", directory.display()),
                    Some(i) => format!("`{}`", i.path().display().to_string()),
                };
                eprintln!("Permission denied reading {}: {}", description, error);
                None
            }
            _ => Some(error),
        }
    }
}

/// A compact immutable representation of the paths within a directory.
#[derive(Clone, PartialEq, Eq)]
pub struct CompactPathTree {
    root: PathBuf,
    path: PathBuf,
}

impl CompactPathTree {
    fn add_item(
        path: &mut PathBuf,
        item: &DirEntry,
        visitor: &mut impl PathVisitor,
    ) -> io::Result<()> {
        if !visitor.filter(&item)? {
            return Ok(());
        }

        visitor.visit(item)?;

        // very important! try to get type before adding anything to the tree:
        // if an error occurs and the visitor opts to ignore it, we don't want
        // to leave the tree in a partially modified state
        let typ = item.file_type()?;

        path.push(item.file_name());
        if typ.is_dir() {
            // as above, make sure we never leave the path in an illegal state
            let r = Self::build_tree(path, &item.path(), visitor);
            path.push(Component::ParentDir.as_os_str());
            r?;
        } else {
            path.push(Component::ParentDir.as_os_str());
        }

        Ok(())
    }

    fn build_tree(
        path: &mut PathBuf,
        dir: &Path,
        visitor: &mut impl PathVisitor,
    ) -> io::Result<()> {
        for item in dir.read_dir()? {
            let item = match item.map_err(|e| visitor.handle_error(e, dir, None)) {
                Ok(i) => i,
                Err(None) => continue,
                Err(Some(e)) => return Err(e),
            };

            if let Err(Some(e)) = Self::add_item(path, &item, visitor)
                .map_err(|e| visitor.handle_error(e, dir, Some(&item)))
            {
                return Err(e);
            }
        }

        Ok(())
    }

    /// Construct a new `CompactPathTree` by doing a depth-first traversal of
    /// the given directory.
    ///
    /// The given visitor is used to determine which items should be included
    /// and what errors are fatal.
    ///
    /// Symbolic links will be stored in the tree, but not followed.
    pub fn new(root: PathBuf, visitor: &mut impl PathVisitor) -> io::Result<Self> {
        let mut path = PathBuf::new();
        Self::build_tree(&mut path, &root, visitor)?;
        path.shrink_to_fit();

        Ok(Self { root, path })
    }

    /// Get the underlying path this tree is represented as.
    pub fn inner(&self) -> &Path {
        &self.path
    }

    /// Get the root path this tree was constructed from.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Get an iterator over the paths stored in this tree.
    ///
    /// The root path isn't included in the output of this iterator, only
    /// its contents are. The paths are iterated in a depth-first traversal of
    /// the tree, with parents being emitted before children. No other
    /// guarantees are made with regards to ordering.
    pub fn iter(&self) -> CompactPathTreeIter {
        self.into_iter()
    }
}

impl<'a> IntoIterator for &'a CompactPathTree {
    type Item = PathBuf;
    type IntoIter = CompactPathTreeIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        CompactPathTreeIter {
            current: self.root.clone(),
            components: self.path.components(),
        }
    }
}

pub struct CompactPathTreeIter<'a> {
    current: PathBuf,
    components: Components<'a>,
}

impl<'a> Iterator for CompactPathTreeIter<'a> {
    type Item = PathBuf;

    fn next(&mut self) -> Option<Self::Item> {
        for c in &mut self.components {
            match c {
                Component::ParentDir => {
                    self.current.pop();
                }
                Component::Normal(p) => {
                    self.current.push(p);
                    return Some(self.current.clone());
                }
                c => unreachable!("illegal component {:?} in path tree", c),
            }
        }

        None
    }
}
