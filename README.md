# compact_path_tree

A simple library for representing an iterable tree of paths efficiently in memory.
Rather than storing a `PathBuf` object for every file, this implementation instead stores a single
giant `PathBuf` and uses relative path components to represent items.
For example, this directory structure...

- outer
  - a
  - b
    - c
    - d
  - e
  
...could be represented as the following path:

```
outer/a/../b/c/../d/e
```

This often saves a significant amount of memory, since every item would otherwise require at least
two machine words to store in addition to the contents of the filename, whereas this representation
reduces it to around four characters per entry.

This approach has obvious limitations, however.
The trees are immutable once constructed, and cannot be sorted in any particular order - the only
guarantee is that they can be iterated to provide a depth-first traversal of the paths it
represents.
