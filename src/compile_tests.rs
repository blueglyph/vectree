#![allow(unused)]
#![cfg(doctest)]

// IMPORTANT: Run the test in +nightly to verify the error codes
//            (Cargo doesn't test them in the stable version...)

mod borrows {
    /// ```compile_fail,E0502
    /// use vectree::VecTree;           // | let a = tree.get_mut(1);
    ///                                 // |         ---- mutable borrow occurs here
    /// let mut tree = VecTree::new();  // | let b = tree.get(2); // cannot borrow `tree` as immutable because it is also borrowed as mutable
    /// let a = tree.get_mut(1);        // |         ^^^^ immutable borrow occurs here
    /// let b = tree.get(2);            // | *a = "0".to_string();
    /// *a = "0".to_string();           // | -- mutable borrow later used here
    /// ```
    fn must_not_compile1() {}

    /// ```compile_fail,E0502
    /// use vectree::VecTree;
    ///                                 // | let a = tree.get(1);
    /// let mut tree = VecTree::new();  // |         ---- immutable borrow occurs here
    /// let a = tree.get(1);            // | let b = tree.get_mut(2); // cannot borrow `tree` as mutable because it is also borrowed as immutable
    /// let b = tree.get_mut(2);        // |         ^^^^^^^^^^^^^^^ mutable borrow occurs here
    /// *b = "0".to_string();           // | *b = "0".to_string();
    /// assert_eq!(a, "a");             // | assert_eq!(a, "a");
    /// ```                             // | ------------------ immutable borrow later used here
    fn must_not_compile2() {}

    /// ```compile_fail,E0502
    /// use vectree::VecTree;
    ///                                          // |
    /// let mut tree = VecTree::<String>::new(); // | let a = tree.get(1);
    /// let a = tree.get(1);                     // |         ---- immutable borrow occurs here
    /// for mut inode in tree.iter_depth_mut() { // | for mut inode in tree.iter_depth_mut() { // cannot borrow `tree` as mutable because it is also borrowed as immutable
    ///     *inode = inode.to_uppercase();       // |                  ^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
    /// }
    /// assert_eq!(a, "A");
    /// ```
    fn must_not_compile3() {}

    /// ```compile_fail,E0502
    /// use std::ops::Deref;
    /// use vectree::VecTree;
    ///
    /// let mut tree = VecTree::<i32>::new();    // | let mut iter = tree.iter_depth_simple();
    /// let mut iter = tree.iter_depth_simple(); // |                ---- immutable borrow occurs here
    /// let a1_borrowed = iter.next().unwrap();  // | let a1_borrowed = iter.next().unwrap();
    /// tree.clear();                            // | tree.clear();
    /// let value = a1_borrowed.deref();         // | ^^^^^^^^^^^^ mutable borrow occurs here
    /// println!("value: {value}");              // | let value = a1_borrowed.deref();
    /// ```
    fn must_not_compile4() {}

    /// ```compile_fail,E0502
    /// use std::ops::Deref;
    /// use vectree::VecTree;
    ///
    /// let mut tree = VecTree::<i32>::new();   // | let mut iter = tree.iter_depth();
    /// let mut iter = tree.iter_depth();       // |                ---- immutable borrow occurs here
    /// let a1_borrowed = iter.next().unwrap(); // | let a1_borrowed = iter.next().unwrap();
    /// tree.clear();                           // | tree.clear();
    /// let value = a1_borrowed.deref();        // | ^^^^^^^^^^^^ mutable borrow occurs here
    /// println!("value: {value}");             // | let value = a1_borrowed.deref();
    /// ```
    fn must_not_compile5() {}

    /// ```compile_fail,E0502
    /// use std::ops::Deref;
    /// use vectree::VecTree;
    ///
    /// let mut tree = VecTree::<i32>::new();
    /// let mut iter = tree.iter_depth();       // |
    /// let _a1 = iter.next();                  // | let mut iter = tree.iter_depth();
    /// let _a2 = iter.next();                  // |                ---- immutable borrow occurs here
    /// let a = iter.next().unwrap();           //
    /// let a1_borrowed = a.iter_children()     // | tree.clear();                // OK: doesn't compile
    ///     .next().unwrap();                   // | ^^^^^^^^^^^^ mutable borrow occurs here
    /// tree.clear();                           // | let value = a1_borrowed.deref();
    /// let value = a1_borrowed.deref();        // |             ----------- immutable borrow later used here
    /// println!("value: {value}");
    /// ```
    fn must_not_compile6() {}

    /// ```compile_fail,E0502
    /// use std::ops::Deref;
    /// use vectree::VecTree;
    ///
    /// let mut tree = VecTree::<String>::new();
    /// let mut iter = tree.iter_depth();       // |
    /// let _a1 = iter.next();                  // | let mut iter = tree.iter_depth();
    /// let _a2 = iter.next();                  // |                ---- immutable borrow occurs here
    /// let a = iter.next().unwrap();           //
    /// let a1_borrowed = a.iter_children()     // | let a1_mut = tree.get_mut(4);
    ///     .next().unwrap();                   // |              ^^^^^^^^^^^^^^^ mutable borrow occurs here
    /// let value1 = a1_borrowed.clone();       // | *a1_mut = "new a1".to_string();
    /// let a1_mut = tree.get_mut(4);           // | let value2 = a1_borrowed.clone();
    /// *a1_mut = "new a1".to_string();         // |              ----------- immutable borrow later used here
    /// let value2 = a1_borrowed.clone();
    /// println!("value: {value1}, {value2}");
    /// ```
    fn must_not_compile7() {}

    /// ```compile_fail,E0499
    /// use std::ops::Deref;
    /// use vectree::VecTree;
    ///
    /// let mut tree = VecTree::<i32>::new();
    /// let mut iter = tree.iter_depth_mut();    // |
    /// let _a1 = iter.next().unwrap();          // | let mut iter = tree.iter_depth_mut();
    /// let _a2 = iter.next();                   // |                ---- first mutable borrow occurs here
    /// let a_mut = iter.next().unwrap();        //
    /// let a1_borrowed = a_mut.iter_children()  // | tree.clear();
    ///     .next().unwrap();                    // | ^^^^ second mutable borrow occurs here
    /// tree.clear();
    /// let value = a1_borrowed.deref();
    /// println!("value: {value}");
    /// ```
    fn must_not_compile8() {}

    /// ```compile_fail,E0502
    /// use vectree::VecTree;
    ///
    /// let mut tree = VecTree::<String>::new();
    /// let mut iter = tree.iter_depth_mut();
    /// let mut a1_mut = iter.next().unwrap();   // |
    /// let a1_borrowed = tree.get(4);           // | let mut iter = tree.iter_depth_mut();
    /// let value1 = a1_borrowed.clone();        // |                ---- mutable borrow occurs here
    /// *a1_mut = "new a1".to_string();          //
    /// let value2 = a1_borrowed.clone();        // | let a1_borrowed = tree.get(4);
    /// println!("value: {value1}, {value2}");   // |                   ^^^^ immutable borrow occurs here
    /// ````
    fn must_not_compile9() {}
}