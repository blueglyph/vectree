[![crate](https://img.shields.io/crates/v/vectree.svg)](https://crates.io/crates/vectree)
[![documentation](https://docs.rs/vectree/badge.svg)](https://docs.rs/vectree)
[![license](https://img.shields.io/badge/License-MIT%202.0-blue.svg)](https://github.com/blueglyph/vectree/blob/master/LICENSE-MIT)
[![license](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/blueglyph/vectree/blob/master/LICENSE-APACHE)

# `vectree` crate

A simple vector-based tree collection that provides flexible immutable and mutable iterators.

The iterators are visiting the nodes in a post-order, depth-first search.


## Examples

```rust
let mut tree = VecTree::new();
let root = tree.add_root("root".to_string());
let a = tree.add(Some(root), "a".to_string());
let _ = tree.add(Some(root), "b".to_string());
let c = tree.add(Some(root), "c".to_string());
tree.add_iter(Some(a), ["a1".to_string(), "a2".to_string()]);
tree.add_iter(Some(c), ["c1", "c2"].map(|s| s.to_string()));

let mut result = String::new();
let mut result_index = vec![];
let mut result_depth = vec![];
for inode in tree.iter_depth_simple() {
    result.push_str(&inode.to_uppercase());
    result.push(',');
    result_index.push(inode.index);
    result_depth.push(inode.depth);
}
assert_eq!(result, "A1,A2,A,B,C1,C2,C,ROOT,");
assert_eq!(result_index, [4, 5, 1, 2, 6, 7, 3, 0]);
assert_eq!(result_depth, [2, 2, 1, 1, 2, 2, 1, 0]);
```

# License

This code is licensed under either [MIT License](https://choosealicense.com/licenses/mit/) or [Apache License 2.0](https://choosealicense.com/licenses/apache-2.0/).
