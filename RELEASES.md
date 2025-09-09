# 0.2.3 (2025-09-09)

- add `children_mut` method to get a mutable reference to an item's children.

# 0.2.2 (2025-08-28)

- add `From<(Option<usize>, IntoIterator<Item=(T, IntoIterator)>)>` implementation that allows to import a VecTree from an iterable set of data (which could be static, depending on `T`).

# 0.2.1 (2025-08-22)

- add `add_from_tree_callback` and `add_from_tree_iter_callback` methods, which are similar to `add_from_tree` and `add_from_tree_iter`, respectively, but add a callback function giving the source index, destination index, and reference of each copied item.

# 0.2.0 (2025-08-20)

- add `num_children()` to iter_depth_simple's node
- separate lifetimes of self and iterator in implementations

# 0.1.6 (2025-05-07)

- fix a few lint warnings

# 0.1.5 (2025-04-03)

- add `VecTree::attach_child` method

# 0.1.4 (2025-03-18)

- initial published crate
- add this release file

# 0.1.3 (2025-02-27)

- use `usize` instead of `&usize` for children references (easier)

# 0.1.2 (2025-02-03)

- add tests and comments

# 0.1.1 (2025-01-30)

- add Default trait to VecTree
- add tests, doc
- add lifetime to `VecTree::iter_depth`, `VecTree::iter_depth_at`, `VecTreeIter::new` 

# 0.1.0 (2025-01-24)

- initial commit
