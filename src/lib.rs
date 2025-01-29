// Copyright 2025 Redglyph
//

use std::cell::{Cell, UnsafeCell};
use std::fmt::{Display, Formatter};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut, Index, IndexMut};
use std::ptr::NonNull;

mod tests;

/// A vector-based tree collection type. Each node is of type [Node<T>].
#[derive(Debug)]
pub struct VecTree<T> {
    nodes: Vec<Node<T>>,
    borrows: Cell<u32>,
    root: Option<usize>
}

/// A node of a [VecTree<T>] collection. It holds a data of type `<T>` and a list
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

impl<'a, T> VecTree<T> {
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
        assert!(index < self.nodes.len());
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
        self.nodes[node_id].children.extend(children_id);
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

    /// Attaches existing children to an existing parent.
    pub fn attach_children<U: IntoIterator<Item = &'a usize>>(&mut self, parent_index: usize, children_index: U) {
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
    /// Note that the depth returned by the iterators are zero-based, and thus `iterator.depth`
    /// is between `0` and `tree.depth()`.
    ///
    /// Returns `None` if the tree has no root.
    pub fn depth(&self) -> Option<u32> {
        self.iter_depth_simple().map(|x| x.depth).max()
    }

    /// Returns a reference to the item stored at the given index.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn get(&self, index: usize) -> &T {
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

    /// Returns an iterator to the item's children, by reference.
    ///
    /// Panics if the index is out of the buffer bounds.
    pub fn iter_children(&self, index: usize) -> impl DoubleEndedIterator<Item = &Node<T>> {
        self.nodes.get(index).unwrap().children.iter().map(|&i| self.nodes.get(i).unwrap())
    }
}

impl<'a, T:'a + Clone> VecTree<T> {
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
    pub fn add_from_tree_iter<U>(&mut self, parent_index: Option<usize>, items: U) -> usize
        where U: Iterator<Item=NodeProxy<'a, T>>
    {
        let mut stack = Vec::<usize>::new();
        for item in items {
            let node = item.deref().clone();
            let num_children = item.num_children();
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

impl<T: Clone> Clone for Node<T> {
    fn clone(&self) -> Self {
        Node {
            data: UnsafeCell::new(unsafe { (*self.data.get()).clone() }),
            children: self.children.clone()
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
pub struct VecTreeIter<TData> {
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

impl<'a, TData: TreeDataIter> Iterator for VecTreeIter<TData> {
    type Item = TData::TProxy;

    fn next(&mut self) -> Option<Self::Item> {
        // post-order depth-first search algorithm, common to all iterators
        while let Some(node_dir) = self.next {
            let index_option = match node_dir {
                VisitNode::Down(index) => {
                    let children = self.data.get_children(index);
                    if children.is_empty() {
                        Some(index.clone())
                    } else {
                        self.depth += 1;
                        self.stack.push(VisitNode::Up(index.clone()));
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

impl<'a, T> VecTree<T> {
    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// its root node.
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference only to that node.
    pub fn iter_depth_simple(&'a self) -> VecTreeIter<IterDataSimple<'a, T>> {
        VecTreeIter::<IterDataSimple<'a, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index [top].
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference only to that node.
    pub fn iter_depth_simple_at(&'a self, top: usize) -> VecTreeIter<IterDataSimple<'a, T>> {
        VecTreeIter::<IterDataSimple<'a, T>>::new(self, Some(top))
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
    pub fn iter_depth(&'a self) -> VecTreeIter<IterData<'a, T>> {
        VecTreeIter::<IterData<'a, T>>::new(&self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index [top].
    ///
    /// The iterator returns a proxy for each node, which gives an immutable reference to that node
    /// and its children with the following methods:
    /// * [NodeProxy::num_children()], to get the number of children
    /// * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
    /// * [NodeProxy::iter_children_simple()], to iterate over the children
    /// * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
    pub fn iter_depth_at(&'a self, top: usize) -> VecTreeIter<IterData<'a, T>> {
        VecTreeIter::<IterData<'a, T>>::new(&self, Some(top))
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// its root node.
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference only to that node.
    pub fn iter_depth_simple_mut(&'a mut self) -> VecTreeIter<IterDataSimpleMut<'a, T>> {
        VecTreeIter::<IterDataSimpleMut<'a, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index [top].
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference only to that node.
    pub fn iter_depth_simple_mut_at(&'a mut self, top: usize) -> VecTreeIter<IterDataSimpleMut<'a, T>> {
        VecTreeIter::<IterDataSimpleMut<'a, T>>::new(self, Some(top))
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
    pub fn iter_depth_mut(&'a mut self) -> VecTreeIter<IterDataMut<'a, T>> {
        VecTreeIter::<IterDataMut<'a, T>>::new(self, self.root)
    }

    /// Post-order, depth-first search iteration over all the nodes of the [VecTree], starting at
    /// the node of index [top].
    ///
    /// The iterator returns a proxy for each node, which gives a mutable reference to that node
    /// and an immutable reference its children with the following methods:
    /// * [NodeProxy::num_children()], to get the number of children
    /// * [NodeProxy::iter_children()], to iterate over the children with a proxy to access their children
    /// * [NodeProxy::iter_children_simple()], to iterate over the children
    /// * [NodeProxy::iter_depth_simple()], to iterate the subtree under the node
    pub fn iter_depth_mut_at(&'a mut self, top: usize) -> VecTreeIter<IterDataMut<'a, T>> {
        VecTreeIter::<IterDataMut<'a, T>>::new(self, Some(top))
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

impl<'a, T> VecTreeIter<IterDataSimple<'a, T>> {
    fn new(tree: &'a VecTree<T>, top: Option<usize>) -> Self {
        VecTreeIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(|id| VisitNode::Down(id)),
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
        assert!(index < self.tree.len());
        unsafe { &(*self.tree.nodes.as_ptr().add(index)).children }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        assert!(index < self.tree.len());
        NodeProxySimple {
            index,
            depth,
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
    data: NonNull<T>,
    _marker: PhantomData<&'a T>
}

impl<T> Deref for NodeProxySimple<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.data.as_ref() }
    }
}

// -- with children

impl<'a, T> VecTreeIter<IterData<'a, T>> {
    fn new(tree: &'a VecTree<T>, top: Option<usize>) -> Self {
        VecTreeIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(|id| VisitNode::Down(id)),
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
        assert!(index < self.tree_size);
        unsafe {
            &self.tree_nodes_ptr.add(index).as_ref().unwrap().children
        }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        assert!(index < self.tree_size);
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

impl<'a, T> NodeProxy<'a, T> {
    /// Gets the number of children of the node.
    pub fn num_children(&self) -> usize {
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.len()
    }

    /// Iterates over the node's children with a proxy to access their children.
    pub fn iter_children(&self) -> impl DoubleEndedIterator<Item=NodeProxy<'_, T>> {
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&index| {
            assert!(index < self.tree_size);
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
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&c| unsafe { &*(*self.tree_node_ptr.add(c)).data.get() })
    }

    /// Iterates the subtree under the node.
    pub fn iter_depth_simple(&'a self) -> VecTreeIter<IterData<'a, T>> {
        VecTreeIter {
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
        unsafe { self.data.as_ref() }
    }
}

// ---------------------------------------------------------------------------------------------
// Mutable iterator

impl<'a, T> VecTreeIter<IterDataSimpleMut<'a, T>> {
    fn new(tree: &'a mut VecTree<T>, top: Option<usize>) -> Self {
        VecTreeIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(|id| VisitNode::Down(id)),
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
        assert!(index < self.tree.len());
        unsafe { &(*self.tree.nodes.as_ptr().add(index)).children }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        assert!(index < self.tree.len());
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
        unsafe { self.data.as_ref() }
    }
}

impl<T> DerefMut for NodeProxySimpleMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.data.as_mut() }
    }
}

// -- with children

impl<'a, T> VecTreeIter<IterDataMut<'a, T>> {
    fn new(tree: &'a mut VecTree<T>, top: Option<usize>) -> Self {
        VecTreeIter {
            stack: Vec::new(),
            depth: 0,
            next: top.map(|id| VisitNode::Down(id)),
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
        assert!(index < self.tree_size);
        unsafe {
            &self.tree_nodes_ptr.add(index).as_ref().unwrap().children
        }
    }

    fn create_proxy(&self, index: usize, depth: u32) -> Self::TProxy {
        let c = self.borrows.get() + 1;
        self.borrows.set(c);
        assert!(index < self.tree_size);
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

impl<'a, T> NodeProxyMut<'a, T> {
    /// Gets the number of children of the node.
    pub fn num_children(&self) -> usize {
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.len()
    }

    /// Iterates over the node's children with a proxy to access their children (immutably).
    pub fn iter_children(&self) -> impl DoubleEndedIterator<Item = NodeProxy<'_, T>> {
        let c = self.borrows.get();
        assert!(c <= 1, "{} extra pending mutable reference(s) on children when requesting immutable references on them", c - 1);
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&index| {
            assert!(index < self.tree_size);
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
        let children = unsafe { &(*self.tree_node_ptr.add(self.index)).children };
        children.iter().map(|&c| unsafe { &*(*self.tree_node_ptr.add(c)).data.get() })
    }

    /// Iterates the subtree under the node (immutably).
    pub fn iter_depth_simple(&'a self) -> VecTreeIter<IterData<'a, T>> {
        VecTreeIter {
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
        unsafe { self.data.as_ref() }
    }
}

impl<T> DerefMut for NodeProxyMut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
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
    type IntoIter = VecTreeIter<IterDataSimple<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_depth_simple()
    }
}

impl<'a, T> IntoIterator for &'a mut VecTree<T> {
    type Item = NodeProxySimpleMut<'a, T>;
    type IntoIter = VecTreeIter<IterDataSimpleMut<'a, T>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_depth_simple_mut()
    }
}

// ---------------------------------------------------------------------------------------------
