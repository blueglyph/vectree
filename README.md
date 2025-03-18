[![crate](https://img.shields.io/crates/v/vectree.svg)](https://crates.io/crates/vectree)
[![documentation](https://docs.rs/vectree/badge.svg)](https://docs.rs/vectree)
[![license](https://img.shields.io/badge/License-MIT%202.0-blue.svg)](https://github.com/blueglyph/vectree/blob/master/LICENSE-MIT)
[![license](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](https://github.com/blueglyph/vectree/blob/master/LICENSE-APACHE)

# `vectree` crate

A simple vector-based tree collection that provides flexible immutable and mutable iterators.

The iterators are visiting the nodes in a post-order, depth-first search.

## Motivation

I needed a simple collection type for a tree structure that allows to iterate with depth-first search, modify the current node (so a mutable iterator) based on its value and the content of its children.

After a mock-up test, a tree stored in a vector seems both quicker for the typical operations and easier to handle with the borrow checker than a pointer-based structure. The usual problem with vector-based trees is the deletion of nodes, but I didn't need that functionality. In one case, I had to replace a few nodes, but it's easy to simply leave the old nodes in the tree without referencing them, so that's what I did.

## Handling Aliasing

The tricky part is to allow a mutable iteration and immutably inspect a node's children in the loop, since I didn't want to use recursivity when scanning the tree. It's performed by using interior mutability in each node, but rather than using `RefCell`, which would do a verification at each access, I'm using an `UnsafeCell` with home rules.

An iterator returns a "proxy" — a fancy way to call a custom smart pointer — instead of a direct reference to the data. This allows to track when the item is dropped, and at the same time, it provides methods to iterate on the node's children.

In short, when there's a mutable iteration on the tree, it's possible to iterate (immutably) over each node's children, but only if there's no other mutable reference in that tree at that moment. So it's not possible to store several mutable references, and then use them to obtain immutable references of their children, which could lead to mutable reference aliasing.

For example, if the node "a" has one child "a1":
- We create a mutable iterator and store the returned mutable references to "a1" (first iteration) and "a" (second iteration).
- We use the "a" reference and iterate over its children, here only "a1". This creates an immutable reference to "a1", but there's already a mutable reference to that node.

In the example above, the method that creates the 2nd immutable iterator would panic because there are other mutable references than the one on "a". But if the mutable reference to "a1" is dropped before creating the immutable iterator, everything is fine. 

## Is It Safe?

I'm not entirely pleased with this system yet, but in the scope of what I needed, I found no aliases. They're either refused at compilation time or detected at runtime, but there is currently no formal proof that all the cases have been accounted for.

## Examples

Building a tree:
```rust
fn build_tree() -> VecTree<String> {
    let mut tree = VecTree::new();
    let root: usize = tree.add_root("root".to_string());
    let a = tree.add(Some(root), "a".to_string());
    let _ = tree.add(Some(root), "b".to_string());
    let c = tree.add(Some(root), "c".to_string());
    tree.add_iter(Some(a), ["a1".to_string(), "a2".to_string()]);
    tree.add_iter(Some(c), ["c1", "c2"].map(|s| s.to_string()));
    tree
}
```

Simple iterator:

```rust
let mut tree = build_tree();
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

More complex iterator that gives access to the node's children:

```rust
let mut tree = build_tree();
for mut inode in tree.iter_depth_mut() {
    // condition: any child j begins with 'c' and has all j's children k (if any) begin with 'c'
    let sub_is_c = inode.iter_children()
        .any(|j| {
            j.to_lowercase().starts_with('c') &&
                j.iter_children().all(|k| k.to_lowercase().starts_with('c'))
        });
    if sub_is_c {
        *inode = inode.to_uppercase();
    }
}
let result = tree_to_string(&tree);
assert_eq!(result, "ROOT(a(a1,a2),b,C(c1,c2))");
```

# Licence

This code is licensed under either [MIT Licence](https://choosealicense.com/licenses/mit/) or [Apache Licence 2.0](https://choosealicense.com/licenses/apache-2.0/).
