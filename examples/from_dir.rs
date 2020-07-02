use compact_path_tree::{CompactPathTree, PathVisitor};
use std::env::args;
use std::fs::DirEntry;
use std::io;
use std::path::PathBuf;
use std::time::Instant;

#[derive(Default, Debug)]
struct Stats {
    files: usize,
    dirs: usize,
    symlinks: usize,
    items: usize,
    bytes: u64,
}

impl PathVisitor for Stats {
    fn visit(&mut self, entry: &DirEntry) -> io::Result<()> {
        // note: on unix this call is non-trivial and can dramatically hurt
        // performance
        let meta = entry.metadata()?;

        if meta.file_type().is_file() {
            self.files += 1;
        } else if meta.file_type().is_dir() {
            self.dirs += 1;
        } else if meta.file_type().is_symlink() {
            self.symlinks += 1;
        }

        self.items += 1;
        self.bytes += meta.len();

        Ok(())
    }
}

fn main() {
    let root = args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| dirs::home_dir().unwrap());

    eprintln!("Constructing tree from {}", root.display());

    let start = Instant::now();
    let mut stats = Stats::default();
    let tree = CompactPathTree::new(root, &mut stats).unwrap();

    eprintln!("Tree complete!");
    eprintln!("Stats: {:?}", stats);
    eprintln!("Total path length: {}", tree.inner().as_os_str().len());
    eprintln!("Constructed in {}ms", start.elapsed().as_millis());

    let mut n = 0;
    for path in tree.iter() {
        path.symlink_metadata()
            .expect("path stored in tree but missing from FS!");
        n += 1;
    }
    assert_eq!(n, stats.items);

    println!("{}", tree.inner().display());
}
