// Copyright 2025 Redglyph
//

//! A simple vector-based tree collection that provides flexible immutable and mutable iterators. See the [VecTree] type for a list of methods.
//!
//! ## Building the tree
//!
//! The nodes are manipulated by their indices, which are returned by the methods adding a node to the tree.
//!
//! Example:
//!
//! ```rust
//! use vectree::VecTree;
//!
//! fn build_tree() -> VecTree<String> {
//!     // use `with_capacity` if you know the number of nodes:
//!     let mut tree = VecTree::new();
//!
//!     // adds the tree root:
//!     let root: usize = tree.add_root("root".to_string());
//!
//!     // adds three children to the root:
//!     let a = tree.add(Some(root), "a".to_string());
//!     let _ = tree.add(Some(root), "b".to_string());
//!     let c = tree.add(Some(root), "c".to_string());
//!
//!     // adds children to existing nodes, "_iter" = from anything iterable
//!     tree.add_iter(Some(a), ["a1".to_string(), "a2".to_string()]);
//!     tree.add_iter(Some(c), ["c1", "c2"].map(|s| s.to_string()));
//!
//!     tree
//! }
//! ```
//!
//! The following methods are used to add nodes:
//! * [VecTree::add]`(&mut self, parent_index: Option<usize>, item: T)`
//! * [VecTree::addc]`(&mut self, parent_index: Option<usize>, item: T, child: T)`
//! * [VecTree::addci]`(&mut self, parent_index: Option<usize>, item: T, child_id: usize)`
//! * [VecTree::addci_iter]`(&mut self, parent_index: Option<usize>, item: T, children_id: IntoIterator<Item = usize>)`
//! * [VecTree::add_iter]`(&mut self, parent_index: Option<usize>, items: IntoIterator<Item = T>)`
//! * [VecTree::addc_iter]`(&mut self, parent_index: Option<usize>, item: T, children: IntoIterator<Item = T>)`
//!
//! The key to the names is
//! * "c" when a child or children can be specified
//! * "i" when indices are used instead of data, if those nodes were previously added to the tree
//! * "iter" when items are provided by anything iterable, like an array or an iterator
//!
//! ## Iterators
//!
//! The iterators are visiting the nodes in a post-order, depth-first search. There are simple and full-fledged iterators
//! * the "simple" iterators give a mutable / immutable reference to each node but not to its children
//! * the "full-fledged" iterators give a mutable / immutable reference to each node and immutable access to its children, with a variety of iterators.
//!
//! List of simple iterators:
//! * [VecTree::iter_depth_simple] (from the top)
//! * [VecTree::iter_depth_simple_mut] (from the top, mutable reference to node)
//! * [VecTree::iter_depth_simple_at] (from a specific node)
//! * [VecTree::iter_depth_simple_at_mut] (from a specific node, mutable reference to node)
//!
//! List of full-fledged iterators:
//! * [VecTree::iter_depth] (from the top)
//! * [VecTree::iter_depth_mut] (from the top, mutable reference to node)
//! * [VecTree::iter_depth_at] (from a specific node)
//! * [VecTree::iter_depth_at_mut] (from a specific node, mutable reference to node)
//!
//! The full-fledged iterators add the following methods to the "proxy" (smart pointer) returned by the iterator:
//! * [NodeProxy::num_children()], to get the number of children
//! * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
//! * [NodeProxy::iter_children_simple()], to iterate over the children
//! * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
//!
//! Examples
//!
//! Simple iterator:
//!
//! ```rust,ignore
//! let mut tree = build_tree();
//! let mut result = String::new();
//! let mut result_index = vec![];
//! let mut result_depth = vec![];
//! for inode in tree.iter_depth_simple() {
//!     result.push_str(&inode.to_uppercase());
//!     result.push(',');
//!     result_index.push(inode.index);
//!     result_depth.push(inode.depth);
//! }
//! assert_eq!(result, "A1,A2,A,B,C1,C2,C,ROOT,");
//! assert_eq!(result_index, [4, 5, 1, 2, 6, 7, 3, 0]);
//! assert_eq!(result_depth, [2, 2, 1, 1, 2, 2, 1, 0]);
//! ```
//!
//! More complex iterator that gives access to the node's children:
//!
//! ```rust,ignore
//! let mut tree = build_tree();
//! for mut inode in tree.iter_depth_mut() {
//!     // condition: any child j begins with 'c' and
//!     //                        all j's children k (if any) begin with 'c'
//!     let sub_is_c = inode.iter_children()
//!         .any(|j| {
//!             j.to_lowercase().starts_with('c') &&
//!                 j.iter_children().all(|k| k.to_lowercase().starts_with('c'))
//!         });
//!     if sub_is_c {
//!         *inode = inode.to_uppercase();
//!     }
//! }
//! let result = tree_to_string(&tree);
//! assert_eq!(result, "ROOT(a(a1,a2),b,C(c1,c2))");
//! ```
//!
//! ## Important limitation
//!
//! The [VecTree] object doesn't provide methods to delete nodes.

use std::cell::{Cell, UnsafeCell};
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr::NonNull;

mod tests;
mod compile_tests;

/// A vector-based tree collection type. Each node is of type [`Node<T>`].
#[derive(Debug)]
pub struct VecTree<T> {
    nodes: Vec<Node<T>>,
    borrows: Cell<u32>,
    root: Option<usize>
}

/// A node of a [`VecTree<T>`] collection. It holds a data of type `<T>` and a list
/// of indices to its children in the tree collection.
#[derive(Debug)]
pub struct Node<T> {
    data: UnsafeCell<T>,
    children: Vec<usize>
}

/// An index holder indicating the direction of the search: up or down. This type is stored
/// in the stack used by the post-order, depth-first search loop.
#[derive(Clone, Copy)]
enum VisitNode<T> {
    Down(T),
    Up(T)
}

// ---------------------------------------------------------------------------------------------

impl<T> VecTree<T> {
    /// Creates a new and empty tree, with no pre-allocated buffer.
    ///
    /// If the number of items is known in advance, prefer the [`VecTree::with_capacity()`] method.
    pub fn new() -> Self {
        VecTree { nodes: Vec::new(), borrows: Cell::new(0), root: None }
    }

    /// Creates a new and empty tree with pre-allocated buffer of the specified initial capacity.
    /// This method should be used if the number of items is known in advance, since it reduces
    /// the number of memory allocation / deallocation that would normally occur when adding
    /// items to a tree created with the [`VecTree::new()`] method.
    ///
    /// `capacity` is not a hard limit; once pre-allocated, it's still possible to add data
    /// beyond the pre-allocated number of items.
    pub fn with_capacity(capacity: usize) -> Self {
        VecTree { nodes: Vec::with_capacity(capacity), borrows: Cell::new(0), root: None }
    }

    /// Returns the index of the tree root item, if it exists.
    pub fn get_root(&self) -> Option<usize> {
        self.root
    }

    /// Sets the root of the tree by specifying its index. The method returns `index` for
    /// convenience.
    ///
    /// Note:
    /// * `index` must be the index of an existing item, otherwise the method panics.
    /// * If the root was already defined, this method will redefine it. If the previous root
    ///   was a parent of the newly defined root, the previous root and all the other items
    ///   that are not below the new root are not accessible by the iterators any more, but
    ///   they remain in the collection, so they may still be accessed with methods like [`VecTree::get()`],
    ///   or referenced as children indices with methods like [`VecTree::addci()`]. However,
    ///   the user is responsible for preserving the integrity of the tree when doing so.
    pub fn set_root(&mut self, index: usize) -> usize {
        assert!(index < self.nodes.len(), "node index {index} doesn't exist");
        self.root = Some(index);
        index
    }

    /// Adds an item and defines it as root of the tree. The method returns the index of the
    /// item.
    ///
    /// Note:
    /// * If the root was already defined, this method will redefine it. If the previous root
    ///   was a parent of the newly defined root, the previous root and all the other items
    ///   that are not below the new root are not accessible by the iterators any more, but
    ///   they remain in the collection, so they may still be accessed with methods like [`VecTree::get()`],
    ///   or referenced as children indices with methods like [`VecTree::addci()`]. However,
    ///   the user is responsible for preserving the integrity of the tree when doing so.
    pub fn add_root(&mut self, item: T) -> usize {
        self.root = Some(self.add(None, item));
        self.root.unwrap()
    }

    /// Adds an item to the tree and returns its index.
    ///
    /// If `parent_index` is provided (not `None`), the item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the item must be attached to
    /// the tree another way.
    pub fn add(&mut self, parent_index: Option<usize>, item: T) -> usize {
        let index = self.nodes.len();
        if let Some(parent_index) = parent_index {
            self.nodes[parent_index].children.push(index);
        }
        let node = Node { data: UnsafeCell::new(item), children: Vec::new() };
        self.nodes.push(node);
        index
    }

    /// Adds an item and its child to the tree, and returns the item's index.
    ///
    /// If `parent_index` is provided (not `None`), the item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the item must be attached to
    /// the tree another way.
    pub fn addc(&mut self, parent_index: Option<usize>, item: T, child: T) -> usize {
        let index = self.add(parent_index, item);
        self.add(Some(index), child);
        index
    }

    /// Adds an item to the tree, attaching an existing child to it, and returns the item's index.
    ///
    /// If `parent_index` is provided (not `None`), the item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the item must be attached to
    /// the tree another way.
    pub fn addci(&mut self, parent_index: Option<usize>, item: T, child_id: usize) -> usize {
        assert!(child_id < self.len(), "child node index {child_id} doesn't exist");
        let node_id = self.add(parent_index, item);
        self.nodes[node_id].children.push(child_id);
        node_id
    }

    /// Adds an item to the tree, attaching existing children to it, and returns the item's index.
    ///
    /// If `parent_index` is provided (not `None`), the item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the item must be attached to
    /// the tree another way.
    pub fn addci_iter<U: IntoIterator<Item = usize>>(&mut self, parent_index: Option<usize>, item: T, children_id: U) -> usize {
        let node_id = self.add(parent_index, item);
        for child_id in children_id {
            assert!(child_id < self.len(), "child node index {child_id} doesn't exist");
            self.nodes[node_id].children.push(child_id);
        }
        node_id
    }

    /// Adds items to the tree and returns their indices.
    ///
    /// If `parent_index` is provided (not `None`), the item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the item must be attached to
    /// the tree another way.
    pub fn add_iter<U: IntoIterator<Item = T>>(&mut self, parent_index: Option<usize>, items: U) -> Vec<usize> {
        let mut indices = Vec::new();
        for item in items {
            indices.push(self.add(parent_index, item));
        }
        indices
    }

    /// Adds an item and its children to the tree, and returns the item's index.
    ///
    /// If `parent_index` is provided (not `None`), the item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the item must be attached to
    /// the tree another way.
    pub fn addc_iter<U: IntoIterator<Item = T>>(&mut self, parent_index: Option<usize>, item: T, children: U) -> usize {
        let index = self.add(parent_index, item);
        self.add_iter(Some(index), children);
        index
    }

    /// Attaches one extra existing child to an existing parent.
    pub fn attach_child(&mut self, parent_index: usize, child_index: usize) {
        self.nodes[parent_index].children.push(child_index);
    }

    /// Attaches extra existing children to an existing parent.
    pub fn attach_children<U: IntoIterator<Item = usize>>(&mut self, parent_index: usize, children_index: U) {
        self.nodes[parent_index].children.extend(children_index);
    }

    /// Returns the number of items in the tree buffer.
    ///
    /// Note that this method only returns the number of items in the tree, as defined by its current root, if
    /// all items are children of the root to some degree. If there are loose items that have no relationship
    /// with the root, the actual number of items (nodes) in the tree can be obtained by counting the iterations.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns `true` if the tree buffer contains no items.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Calculates the tree depth, which is the maximum number of levels (not including the root).
    ///
    /// Notes:
    /// * The depth returned by the iterators are zero-based, and thus `iterator.depth` is between `0` and `tree.depth()`.
    /// * This method iterates over all the nodes, so it's not time-effective.
    ///
    /// Returns `None` if the tree has no root.
    pub fn depth(&self) -> Option<u32> {
        self.iter_depth_simple().map(|x| x.depth).max()
    }

    /// Returns a reference to the item stored at the given index.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn get(&self, index: usize) -> &T {
        // SAFETY: The access to the `UnsafeCell<T> data` field is secured by the compiler:
        //         the method can't be called if a mutable borrow is alive (either given by .get_mut or
        //         by a NodeProxyMut)
        unsafe { &*self.nodes.get(index).unwrap().data.get() }
    }

    /// Returns a mutable reference to the item stored at the given index.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn get_mut(&mut self, index: usize) -> &mut T {
        self.nodes.get_mut(index).unwrap().data.get_mut()
    }

    /// Returns a reference to the item's children.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn children(&self, index: usize) -> &[usize] {
        self.nodes.get(index).unwrap().children.as_slice()
    }

    /// Returns a mutable reference to the item's children.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn children_mut(&mut self, index: usize) -> &mut Vec<usize> {
        &mut self.nodes.get_mut(index).unwrap().children
    }

    /// Returns an iterator to the item's children, by reference.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn iter_children(&self, index: usize) -> impl DoubleEndedIterator<Item = &Node<T>> {
        self.nodes.get(index).unwrap().children.iter().map(|&i| self.nodes.get(i).unwrap())
    }
}

impl<T: Clone> VecTree<T> {
    /// Adds items from another `VecTree` and returns the index of the top item. This method
    /// can be used to copy another tree or part of another tree into the current one.
    ///
    /// The items are cloned from the other tree. If `top` is not `None`, it contains the index of
    /// the top element that is copied from the other tree. If `top` is `None`, the whole tree is
    /// added.
    ///
    /// If `parent_index` is provided (not `None`), the top item is added to the parent's list of children.
    /// If that parent doesn't exist, or in other words, if the value of `parent_index` is too big for the
    /// buffer size, the method panics. If `parent_index` is `None`, the top item must be attached to
    /// the current tree another way.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vectree::VecTree;
    /// let mut tree = VecTree::new();
    /// let root = tree.add_root("root".to_string());
    /// let a = tree.add(Some(root), "a".to_string());
    /// let b = tree.add(Some(root), "b".to_string());
    /// let _ = tree.add(Some(root), "c".to_string());
    /// tree.add_iter(Some(a), ["a1".to_string(), "a2".to_string()]);
    /// // => tree:  root(a(a1, a2), b, c)
    /// let other = tree.clone();
    /// tree.add_from_tree(Some(b), &other, Some(a));
    /// // => tree: root(a(a1, a2), b(a(a1, a2)), c)
    /// //                           ^^^^^^^^^^^
    /// ```
    pub fn add_from_tree(&mut self, parent_index: Option<usize>, tree: &VecTree<T>, top: Option<usize>) -> usize {
        self.add_from_tree_iter(parent_index, tree.iter_depth_at(top.unwrap_or_else(|| tree.get_root().unwrap())))
    }

    /// Adds items from a `VecTree` iterator and returns the index of the top item. This method
    /// can be used to copy another tree or part of another tree into the current one.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use vectree::VecTree;
    /// let mut tree = VecTree::new();
    /// let root = tree.add_root("root".to_string());
    /// let a = tree.add(Some(root), "a".to_string());
    /// let b = tree.add(Some(root), "b".to_string());
    /// let _ = tree.add(Some(root), "c".to_string());
    /// tree.add_iter(Some(a), ["a1".to_string(), "a2".to_string()]);
    /// // => tree:  root(a(a1, a2), b, c)
    /// let other = tree.clone();
    /// tree.add_from_tree_iter(Some(b), other.iter_depth_at(a));
    /// // => tree: root(a(a1, a2), b(a(a1, a2)), c)
    /// //                           ^^^^^^^^^^^
    /// ```
    pub fn add_from_tree_iter<'a, U>(&mut self, parent_index: Option<usize>, items: U) -> usize
    where
        U: IntoIterator<Item=NodeProxy<'a, T>>,
        T: 'a
    {
        self.add_from_tree_iter_callback(parent_index, items, move |_, _, _| {})
    }

    /// Adds items from another `VecTree` and returns the index of the top item. This method
    /// can be used to copy another tree or part of another tree into the current one.
    ///
    /// For further details, see [VecTree::add_from_tree].
    ///
    /// The callback function, `f(to, from, item)` gives the following parameters:
    /// * `to: usize` is the index where the new item will be stored in the destination tree
    /// * `from: usize` is the index where the original item is in the source tree
    /// * `item: &T` is a reference to the item in the source tree
    ///
    /// # Example
    ///
    /// ```
    /// use vectree::VecTree;
    /// # fn main() {
    /// let mut tree = VecTree::new();
    /// let root = tree.add_root("root".to_string());
    /// let a = tree.add(Some(root), "a".to_string());
    /// tree.add(Some(a), "a1".to_string());
    /// tree.add(Some(root), "b".to_string());
    /// let mut result = Vec::<(usize, usize, String)>::new();
    /// let mut other = VecTree::new();
    /// other.add_from_tree_callback(
    ///     None,
    ///     &tree,
    ///     Some(a),
    ///     |to, from, item| result.push((to, from, item.clone())) );
    /// assert_eq!(result, vec![(0, 2, "a1".to_string()), (1, 1, "a".to_string())]);
    /// # }
    /// ```
    pub fn add_from_tree_callback<'a, F>(&mut self, parent_index: Option<usize>, tree: &'a VecTree<T>, top: Option<usize>, f: F) -> usize
    where
        F: FnMut(usize, usize, &T),
        T: 'a
    {
        self.add_from_tree_iter_callback(parent_index, tree.iter_depth_at(top.unwrap_or_else(|| tree.get_root().unwrap())), f)
    }

    /// Adds items from a `VecTree` iterator and returns the index of the top item. This method
    /// can be used to copy another tree or part of another tree into the current one.
    ///
    /// The callback function, `f(to, from, item)` gives the following parameters:
    /// * `to: usize` is the index where the new item will be stored in the destination tree
    /// * `from: usize` is the index where the original item is in the source tree
    /// * `item: &T` is a reference to the item in the source tree
    ///
    ///
    /// # Example
    ///
    /// ```
    /// use vectree::VecTree;
    /// # fn main() {
    /// let mut tree = VecTree::new();
    /// let root = tree.add_root("root".to_string());
    /// let a = tree.add(Some(root), "a".to_string());
    /// tree.add(Some(a), "a1".to_string());
    /// tree.add(Some(root), "b".to_string());
    /// let mut result = Vec::<(usize, usize, String)>::new();
    /// let mut other = VecTree::new();
    /// other.add_from_tree_iter_callback(
    ///     None,
    ///     tree.iter_depth_at(a),
    ///     |to, from, item| result.push((to, from, item.clone())) );
    /// assert_eq!(result, vec![(0, 2, "a1".to_string()), (1, 1, "a".to_string())]);
    /// # }
    /// ```
    pub fn add_from_tree_iter_callback<'a, U, F>(&mut self, parent_index: Option<usize>, items: U, mut f: F) -> usize
    where
        U: IntoIterator<Item=NodeProxy<'a, T>>,
        T: 'a,
        F: FnMut(usize, usize, &T),
    {
        let mut stack = Vec::<usize>::new();
        for item in items {
            let node = item.deref().clone();
            let num_children = item.num_children();
            f(self.nodes.len(), item.index, item.deref());
            let index = if num_children > 0 {
                let children = stack.split_off(stack.len() - num_children);
                self.addci_iter(None, node, children)
            } else {
                self.add(None, node)
            };
            stack.push(index);
        }
        assert_eq!(stack.len(), 1, "something is wrong with the structure of the provided items");
        let index = stack.pop().unwrap();
        if let Some(parent) = parent_index {
            self.nodes[parent].children.push(index);
        }
        index
    }
}

impl<T> Node<T> {
    /// Returns `true` if the node has children.
    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// Returns a reference slice to the children's indices.
    pub fn children(&self) -> &[usize] {
        &self.children
    }
}

impl<T> Index<usize> for VecTree<T> {
    type Output = Node<T>;

    fn index(&self, index: usize) -> &Self::Output {
        self.nodes.get(index).unwrap()
    }
}

impl<T> IndexMut<usize> for VecTree<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.nodes.get_mut(index).unwrap()
    }
}

impl<T: Clone> Clone for VecTree<T> {
    fn clone(&self) -> Self {
        VecTree {
            nodes: self.nodes.clone(),
            borrows: Cell::new(0),
            root: self.root
        }
    }
}

impl<T> Default for VecTree<T> {
    fn default() -> Self {
        VecTree::new()
    }
}

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node {
            // SAFETY: We're cloning, so there is no reference to the newly created field.
            data: UnsafeCell::new(unsafe { (*self.data.get()).clone() }),
            children: self.children.clone()
        }
    }
}

// This trait is used as a bound for both usize and &usize. It would be otherwise impossible
// to implement From for iterators on both types (except by using Borrow, which produces
// the same optimized code but is conceptually contradictory with what we do).
trait IntoUsize {
    fn into_usize(self) -> usize;
}

impl IntoUsize for usize {
    fn into_usize(self) -> usize { self }
}

impl IntoUsize for &usize {
    fn into_usize(self) -> usize { *self }
}

impl<T, A, C> From<(Option<usize>, A)> for VecTree<T>
where
    A: IntoIterator<Item=(T, C)>,
    C: IntoIterator<Item: IntoUsize>,
{
    /// Creates a [VecTree] from a tuple `(r, array)`, where
    /// * `r` is an optional root index
    /// * `array` is any collection that can be converted into an iterator of `(T, &[usize])`,
    ///   `T` being the node value at that index and the slice being the enumeration of its
    ///   children indices.
    ///
    /// ## Example
    /// ```rust
    /// # use vectree::VecTree;
    /// let tree = VecTree::from((
    ///     Some(0),
    ///     vec![
    ///         ("root", vec![1, 2]),
    ///         ("a",    vec![3, 4]),
    ///         ("b",    vec![]),
    ///         ("a.1",  vec![]),
    ///         ("a.2",  vec![]),
    ///     ]
    /// ));
    /// let str = tree.iter_depth_simple()
    ///     .map(|n| format!("{}:{}", n.depth, *n))
    ///     .collect::<Vec<_>>()
    ///     .join(", ");
    /// assert_eq!(str, "2:a.1, 2:a.2, 1:a, 1:b, 0:root");
    /// ```
    fn from((root, nodes): (Option<usize>, A)) -> Self {
        VecTree {
            nodes: nodes.into_iter()
                .map(|(value, children)| Node { data: UnsafeCell::new(value), children: children.into_iter().map(|c| c.into_usize()).collect() })
                .collect(),
            borrows: Cell::new(0),
            root,
        }
    }
}

impl<T: Display> Display for VisitNode<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            VisitNode::Down(v) => write!(f, "D({v})"),
            VisitNode::Up(v) => write!(f, "U({v})"),
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Iterators
//
// Since the main depth-first search loop is the same for all iterators, we use the `TData` generic parameter
// that implements the `TreeDataIter` trait. There are four types of iterator:
// * IterDataSimple:   each iteration returns a NodeProxySimple
// * IterDataSimpleMut:                         NodeProxySimpleMut
// * IterData:                                  NodeProxy
// * IterDataMut:                               NodeProxyMut
//
// NodeProxySimple / NodeProxySimpleMut allow to access the data.
// NodeProxy / NodeProxy allow to access the data and the node's children, and it also allows to
// iterate over the children or even the subtree with another embedded depth-first search, with
// that node as root.

/// A [VecTree] post-order, depth-first search iterator.
pub struct VecTreePoDfsIter<TData> {
    stack: Vec<VisitNode<usize>>,
    depth: u32,
    next: Option<VisitNode<usize>>,
    data: TData
}

/// Implements methods used by the depth-first search algorithm and which depends on the
/// type of iterator: simple or full-fledged (allowing to search each node's children),
/// immutable or mutable.
pub trait TreeDataIter {
    type TProxy;

    /// Gets a reference slice to the node's children indices.
    fn get_children(&self, index: usize) -> &[usize];

    /// Creates the proxy returned by each iteration. The proxy is used to access the
    /// tree node and, when a full-fledged iterator is used, the nodes below it.
    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy;
}

impl<TData: TreeDataIter> Iterator for VecTreePoDfsIter<TData> {
    type Item = TData::TProxy;

    fn next(&mut self) -> Option<Self::Item> {
        // post-order depth-first search algorithm, common to all iterators
        while let Some(node_dir) = self.next {
            let index_option = match node_dir {
                VisitNode::Down(index) => {
                    let children = self.data.get_children(index);
                    if children.is_empty() {
                        Some(index)
                    } else {
                        self.depth += 1;
                        self.stack.push(VisitNode::Up(index));
                        for index in children.iter().rev() {
                            self.stack.push(VisitNode::Down(*index));
                        }
                        None
                    }
                }
                VisitNode::Up(index) => {
                    self.depth -= 1;
                    Some(index)
                }
            };
            self.next = self.stack.pop();
            if let Some(index) = index_option {
                return Some(self.data.create_proxy(index, self.depth));
            }
        }
        None
    }
}

impl<'a: 'i,'i, T> VecTree<T> {
    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// its root node.
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference only to that node.
    pub fn iter_depth_simple(&'a self) -> VecTreePoDfsIter<IterDataSimple<'i, T>> {
        VecTreePoDfsIter::<IterDataSimple<'i, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index `top`.
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference only to that node.
    pub fn iter_depth_simple_at(&'a self, top: usize) -> VecTreePoDfsIter<IterDataSimple<'i, T>> {
        VecTreePoDfsIter::<IterDataSimple<'i, T>>::new(self, Some(top))
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// its root node.
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference to that node
    /// and its children with the following methods:
    /// * [NodeProxy::num_children()], to get the number of children
    /// * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
    /// * [NodeProxy::iter_children_simple()], to iterate over the children
    /// * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
    pub fn iter_depth(&'a self) -> VecTreePoDfsIter<IterData<'i, T>> {
        VecTreePoDfsIter::<IterData<'i, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index `top`.
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference to that node
    /// and its children with the following methods:
    /// * [NodeProxy::num_children()], to get the number of children
    /// * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
    /// * [NodeProxy::iter_children_simple()], to iterate over the children
    /// * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
    pub fn iter_depth_at(&'a self, top: usize) -> VecTreePoDfsIter<IterData<'i, T>> {
        VecTreePoDfsIter::<IterData<'i, T>>::new(self, Some(top))
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// its root node.
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference only to that node.
    pub fn iter_depth_simple_mut(&'a mut self) -> VecTreePoDfsIter<IterDataSimpleMut<'i, T>> {
        VecTreePoDfsIter::<IterDataSimpleMut<'i, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index `top`.
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference only to that node.
    pub fn iter_depth_simple_at_mut(&'a mut self, top: usize) -> VecTreePoDfsIter<IterDataSimpleMut<'i, T>> {
        VecTreePoDfsIter::<IterDataSimpleMut<'i, T>>::new(self, Some(top))
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// its root node.
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference to that node
    /// and an immutable reference its children with the following methods:
    /// * [NodeProxy::num_children()], to get the number of children
    /// * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
    /// * [NodeProxy::iter_children_simple()], to iterate over the children
    /// * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
    pub fn iter_depth_mut(&'a mut self) -> VecTreePoDfsIter<IterDataMut<'i, T>> {
        VecTreePoDfsIter::<IterDataMut<'i, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index `top`.
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference to that node
    /// and an immutable reference its children with the following methods:
    /// * [NodeProxy::num_children()], to get the number of children
    /// * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
    /// * [NodeProxy::iter_children_simple()], to iterate over the children
    /// * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
    pub fn iter_depth_at_mut(&'a mut self, top: usize) -> VecTreePoDfsIter<IterDataMut<'i, T>> {
        VecTreePoDfsIter::<IterDataMut<'i, T>>::new(self, Some(top))
    }

    /// Clears the tree content.
    pub fn clear(&mut self) {
        // should never happen, since the compiler wouldn't allow another mutable borrow (required by this method):
        assert_eq!(self.borrows.get(), 0, "must drop all iterator's node references before clearing a VecTree");
        self.nodes.clear();
        self.root = None;
    }
}

// ---------------------------------------------------------------------------------------------
// Immutable iterator

impl<'a: 'i, 'i, T> VecTreePoDfsIter<IterDataSimple<'i, T>> {
    fn new(tree: &'a VecTree<T>, top: Option<usize>) -> Self {
        VecTreePoDfsIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(VisitNode::Down),
            data: IterDataSimple { tree },
        }
    }
}

/// A structure used by simple [VecTree] iterators that give immutable access to each node
/// but not to its children.
pub struct IterDataSimple<'a, T> {
    tree: &'a VecTree<T>,
}

impl<'a, T> TreeDataIter for IterDataSimple<'a, T> {
    type TProxy = NodeProxySimple<'a, T>;

    fn get_children(&self, index: usize) -> &[usize] {
        // SAFETY: We manually check `index`.
        assert!(index < self.tree.len(), "node index {index} doesn't exist");
        unsafe { &(*self.tree.nodes.as_ptr().add(index)).children }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        // SAFETY: - We manually check `index`, so the data reference can't be null.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        assert!(index < self.tree.len(), "node index {index} doesn't exist");
        NodeProxySimple {
            index,
            depth,
            num_children: unsafe { &(*self.tree.nodes.as_ptr().add(index)).children }.len(),
            data: unsafe { NonNull::new_unchecked((*self.tree.nodes.as_ptr().add(index)).data.get()) },
            _marker: PhantomData
        }
    }
}

/// A proxy returned by simple [VecTree] iterators that give immutable access to each node
/// but not to its children.
pub struct NodeProxySimple<'a, T> {
    pub index: usize,
    pub depth: u32,
    num_children: usize,
    data: NonNull<T>,
    _marker: PhantomData<&'a T>
}

impl<T> NodeProxySimple<'_, T> {
    /// Gets the number of children of the node.
    pub fn num_children(&self) -> usize {
        self.num_children
    }
}

impl<T> Deref for NodeProxySimple<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: - The data lives as long as the proxy.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        unsafe { self.data.as_ref() }
    }
}

// -- with children

impl<'a, T> VecTreePoDfsIter<IterData<'a, T>> {
    fn new(tree: &'a VecTree<T>, top: Option<usize>) -> Self {
        VecTreePoDfsIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(VisitNode::Down),
            data: IterData {
                tree_nodes_ptr: tree.nodes.as_ptr(),
                tree_size: tree.nodes.len(),
                _marker: PhantomData
            },
        }
    }
}

/// A structure used by full-fledged [VecTree] iterators that give immutable access to each node,
/// its children, and the whole subtree under that node.
pub struct IterData<'a, T> {
    tree_nodes_ptr: *const Node<T>,
    tree_size: usize,
    _marker: PhantomData<&'a T>
}

impl<'a, T> TreeDataIter for IterData<'a, T> {
    type TProxy = NodeProxy<'a, T>;

    fn get_children(&self, index: usize) -> &[usize] {
        // SAFETY: We manually check `index`.
        assert!(index < self.tree_size, "node index {index} doesn't exist");
        unsafe {
            &self.tree_nodes_ptr.add(index).as_ref().unwrap().children
        }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        // SAFETY: - We manually check `index`, so the data reference can't be null.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        assert!(index < self.tree_size, "node index {index} doesn't exist");
        NodeProxy {
            index,
            depth,
            data: unsafe { NonNull::new_unchecked((*self.tree_nodes_ptr.add(index)).data.get()) },
            tree_node_ptr: self.tree_nodes_ptr,
            tree_size: self.tree_size,
            _marker: PhantomData
        }
    }
}

/// A proxy returned by full-fledged [VecTree] iterators that give immutable access to each node,
/// its children, and the whole subtree under that node.
pub struct NodeProxy<'a, T> {
    pub index: usize,
    pub depth: u32,
    data: NonNull<T>,
    tree_node_ptr: *const Node<T>,
    tree_size: usize,
    _marker: PhantomData<&'a T>
}

impl<'a: 'i, 'i, T> NodeProxy<'a, T> {
    /// Gets the number of children of the node.
    pub fn num_children(&self) -> usize {
        // SAFETY: `self.index` has been verified when the proxy was created.
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.len()
    }

    /// Iterates over the node's children with a proxy to access their children.
    pub fn iter_children(&self) -> impl DoubleEndedIterator<Item=NodeProxy<'_, T>> {
        // SAFETY: - `self.index` has been verified when the proxy was created.
        //         - The children indices have been verified when they were added.
        //           (If an index was bad, it would have been detected before anyway)
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&index| {
            assert!(index < self.tree_size, "node index {index} doesn't exist");
            NodeProxy {
                index,
                depth: self.depth + 1,
                data: unsafe { NonNull::new_unchecked((*self.tree_node_ptr.add(index)).data.get()) },
                tree_node_ptr: self.tree_node_ptr,
                tree_size: self.tree_size,
                _marker: PhantomData,
            }
        })
    }

    /// Iterates over the node's children.
    pub fn iter_children_simple(&self) -> impl DoubleEndedIterator<Item=&T> {
        // SAFETY: - `self.index` has been verified when the proxy was created.
        //         - The children indices have been verified when they were added.
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&c| unsafe { &*(*self.tree_node_ptr.add(c)).data.get() })
    }

    /// Iterates the subtree under the node.
    pub fn iter_depth_simple(&'a self) -> VecTreePoDfsIter<IterData<'i, T>> {
        VecTreePoDfsIter {
            stack: Vec::new(),
            depth: 0,
            next: Some(VisitNode::Down(self.index)),
            data: IterData {
                tree_nodes_ptr: self.tree_node_ptr,
                tree_size: self.tree_size,
                _marker: PhantomData
            },
        }
    }
}

impl<T> Deref for NodeProxy<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: - The data lives as long as the proxy.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        unsafe { self.data.as_ref() }
    }
}

// ---------------------------------------------------------------------------------------------
// Mutable iterator

impl<'a, T> VecTreePoDfsIter<IterDataSimpleMut<'a, T>> {
    fn new(tree: &'a mut VecTree<T>, top: Option<usize>) -> Self {
        VecTreePoDfsIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(VisitNode::Down),
            data: IterDataSimpleMut { tree },
        }
    }
}

/// A structure used by simple [VecTree] iterators that give mutable access to each node
/// but no access to its children.
pub struct IterDataSimpleMut<'a, T> {
    tree: &'a mut VecTree<T>,
}

impl<'a, T> TreeDataIter for IterDataSimpleMut<'a, T> {
    type TProxy = NodeProxySimpleMut<'a, T>;

    fn get_children(&self, index: usize) -> &[usize] {
        // SAFETY: We manually check `index`.
        assert!(index < self.tree.len(), "node index {index} doesn't exist");
        unsafe { &(*self.tree.nodes.as_ptr().add(index)).children }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        // SAFETY: - We manually check `index`, so the data reference can't be null.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        assert!(index < self.tree.len(), "node index {index} doesn't exist");
        NodeProxySimpleMut {
            index,
            depth,
            data: unsafe { NonNull::new_unchecked((*self.tree.nodes.as_ptr().add(index)).data.get()) },
            _marker: PhantomData
        }
    }
}

/// A proxy returned by simple [VecTree] iterators that give mutable access to each node
/// but no access to its children.
pub struct NodeProxySimpleMut<'a, T> {
    pub index: usize,
    pub depth: u32,
    data: NonNull<T>,
    _marker: PhantomData<&'a mut T>     // must be invariant for T
}

impl<T> Deref for NodeProxySimpleMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: - The data lives as long as the proxy.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        unsafe { self.data.as_ref() }
    }
}

impl<T> DerefMut for NodeProxySimpleMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: - The data lives as long as the proxy.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           immutable borrow is possible while it's alive.
        unsafe { self.data.as_mut() }
    }
}

// -- with children

impl<'a, T> VecTreePoDfsIter<IterDataMut<'a, T>> {
    fn new(tree: &'a mut VecTree<T>, top: Option<usize>) -> Self {
        VecTreePoDfsIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(VisitNode::Down),
            data: IterDataMut {
                tree_nodes_ptr: tree.nodes.as_mut_ptr(),
                tree_size: tree.nodes.len(),
                borrows: &tree.borrows,
                _marker: PhantomData
            },
        }
    }
}

/// A structure used by full-fledged [VecTree] iterators that give mutable access to each node,
/// and also immutable access to its children and the whole subtree under that node.
pub struct IterDataMut<'a, T> {
    tree_nodes_ptr: *mut Node<T>,
    tree_size: usize,
    borrows: &'a Cell<u32>,
    _marker: PhantomData<&'a mut T>     // must be invariant for T
}

impl<'a, T> TreeDataIter for IterDataMut<'a, T> {
    type TProxy = NodeProxyMut<'a, T>;

    fn get_children(&self, index: usize) -> &[usize] {
        // SAFETY: We manually check `index`.
        assert!(index < self.tree_size, "node index {index} doesn't exist");
        unsafe {
            &self.tree_nodes_ptr.add(index).as_ref().unwrap().children
        }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        // IterDataMut can spawn immutable iterators, so we keep track of how many mutable proxies (which
        // work as smart pointers) are alive. If more than one is alive, it is forbidden to spawn an
        // immutable iterator, since it would violate the aliasing rule.
        let c = self.borrows.get() + 1;
        self.borrows.set(c);
        // SAFETY: - We manually check `index`, so the data reference can't be null.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        assert!(index < self.tree_size, "node index {index} doesn't exist");
        NodeProxyMut {
            index,
            depth,
            data: unsafe { NonNull::new_unchecked((*self.tree_nodes_ptr.add(index)).data.get()) },
            tree_node_ptr: self.tree_nodes_ptr,
            tree_size: self.tree_size,
            borrows: self.borrows,
            _marker: PhantomData
        }
    }
}

/// A proxy returned by full-fledged [VecTree] iterators that give mutable access to each node,
/// and also immutable access to its children and the whole subtree under that node.
pub struct NodeProxyMut<'a, T> {
    pub index: usize,
    pub depth: u32,
    data: NonNull<T>,
    tree_node_ptr: *const Node<T>,
    tree_size: usize,
    borrows: &'a Cell<u32>,
    _marker: PhantomData<&'a mut T>     // must be invariant for T
}

impl<'a: 'i, 'i, T> NodeProxyMut<'a, T> {
    /// Gets the number of children of the node.
    pub fn num_children(&self) -> usize {
        // SAFETY: `self.index` has been verified when the proxy was created.
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.len()
    }

    /// Iterates over the node's children with a proxy to access their children (immutably).
    pub fn iter_children(&self) -> impl DoubleEndedIterator<Item = NodeProxy<'_, T>> {
        // SAFETY: - We manually check that no mutable borrow is alive before handing a reference to the content of `UnsafeCell<T> data`.
        //         - While such a reference (immutable borrow) is alive, the compiler doesn't allow any immutable borrow on the VecTree.
        //         - `self.index` has been verified when the proxy was created.
        //         - The children indices have been verified when they were added.
        let c = self.borrows.get();
        assert!(c <= 1, "{} extra pending mutable reference(s) on children when requesting immutable references on them", c - 1);
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&index| {
            assert!(index < self.tree_size, "node index {index} doesn't exist");
            NodeProxy {
                index,
                depth: self.depth + 1,
                data: unsafe { NonNull::new_unchecked((*self.tree_node_ptr.add(index)).data.get()) },
                tree_node_ptr: self.tree_node_ptr,
                tree_size: self.tree_size,
                _marker: PhantomData,
            }
        })
    }

    /// Iterates over the node's children (immutably).
    pub fn iter_children_simple(&self) -> impl DoubleEndedIterator<Item=&T> {
        // SAFETY: - `self.index` has been verified when the proxy was created.
        //         - The children indices have been verified when they were added.
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&c| unsafe { &*(*self.tree_node_ptr.add(c)).data.get() })
    }

    /// Iterates the subtree under the node (immutably).
    pub fn iter_depth_simple(&'a self) -> VecTreePoDfsIter<IterData<'i, T>> {
        VecTreePoDfsIter {
            stack: Vec::new(),
            depth: 0,
            next: Some(VisitNode::Down(self.index)),
            data: IterData {
                tree_nodes_ptr: self.tree_node_ptr,
                tree_size: self.tree_size,
                _marker: PhantomData
            },
        }
    }
}

impl<T> Deref for NodeProxyMut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: - The data lives as long as the proxy.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           mutable borrow is possible while it's alive.
        unsafe { self.data.as_ref() }
    }
}

impl<T> DerefMut for NodeProxyMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: - The data lives as long as the proxy.
        //         - The borrow returned by this method has the same lifetime as self, so no
        //           immutable borrow is possible while it's alive.
        unsafe { self.data.as_mut() }
    }
}

impl<T> Drop for NodeProxyMut<'_, T> {
    fn drop(&mut self) {
        let c = self.borrows.get() - 1;
        self.borrows.set(c);
    }
}

// ---------------------------------------------------------------------------------------------
// Shortcuts

impl<'a, T> IntoIterator for &'a VecTree<T> {
    type Item = NodeProxySimple<'a, T>;
    type IntoIter = VecTreePoDfsIter<IterDataSimple<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_depth_simple()
    }
}

impl<'a, T> IntoIterator for &'a mut VecTree<T> {
    type Item = NodeProxySimpleMut<'a, T>;
    type IntoIter = VecTreePoDfsIter<IterDataSimpleMut<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_depth_simple_mut()
    }
}

// ---------------------------------------------------------------------------------------------
