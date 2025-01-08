use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicU32, Ordering},
};

use crate::prelude::ExpandElement;

use super::{Item, Matrix, Variable, VariableKind};

/// An allocator for local variables of a kernel.
///
/// A local variable is unique to a unit. That is, each unit have their own copy of a local variable.
/// There are three types of local variables based on their capabilities.
///     - An immutable local variable is obtained by calling [Allocator::create_local].
///     - A mutable local variable is obtained by calling [Allocator::create_local_mut]. The allocator will reuse
///       previously defined mutable variables if possible.
///     - A restricted mutable local variable is obtained by calling [Allocator::create_local_restricted]. This a is
///       mutable variable that cannot be reused. This is mostly used for loop indices.
///
/// # Performance tips
///
/// In order, prefer immutable local variables, then mutable, then restricted.
///
/// To enable many compiler optimizations, it is prefered to use the [static single-assignment] strategy for immutable variables.
/// That is, each variable must be declared and used exactly once.
///
/// [static single-assignment](https://en.wikipedia.org/wiki/Static_single-assignment_form)
#[derive(Clone, Debug, Default)]
pub struct Allocator {
    local_mut_pool: Rc<RefCell<HashMap<Item, Vec<ExpandElement>>>>,
    next_id: Rc<AtomicU32>,
}

impl PartialEq for Allocator {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.local_mut_pool, &other.local_mut_pool)
            && Rc::ptr_eq(&self.next_id, &other.next_id)
    }
}

impl Allocator {
    /// Create a new immutable local variable of type specified by `item`.
    pub fn create_local(&self, item: Item) -> ExpandElement {
        let id = self.new_local_index();
        let local = VariableKind::LocalConst { id };
        ExpandElement::Plain(Variable::new(local, item))
    }

    /// Create a new mutable local variable of type specified by `item`.
    /// Try to reuse a previously defined but unused mutable variable if possible.
    /// Else, this define a new variable.
    pub fn create_local_mut(&self, item: Item) -> ExpandElement {
        if item.elem.is_atomic() {
            self.create_local_restricted(item)
        } else {
            self.reuse_local_mut(item)
                .unwrap_or_else(|| ExpandElement::Managed(self.add_local_mut(item)))
        }
    }

    /// Create a new mutable restricted local variable of type specified by `item`.
    pub fn create_local_restricted(&self, item: Item) -> ExpandElement {
        let id = self.new_local_index();
        let local = VariableKind::LocalMut { id };
        ExpandElement::Plain(Variable::new(local, item))
    }

    pub fn create_local_array(&self, item: Item, array_size: u32) -> ExpandElement {
        let id = self.new_local_index();
        let local_array = Variable::new(
            VariableKind::LocalArray {
                id,
                length: array_size,
            },
            item,
        );
        ExpandElement::Plain(local_array)
    }

    /// Create a slice variable
    pub fn create_slice(&self, item: Item) -> ExpandElement {
        let id = self.new_local_index();
        let variable = Variable::new(VariableKind::Slice { id }, item);
        ExpandElement::Plain(variable)
    }

    /// Create a matrix variable
    pub fn create_matrix(&self, matrix: Matrix) -> ExpandElement {
        let id = self.new_local_index();
        let variable = Variable::new(
            VariableKind::Matrix { id, mat: matrix },
            Item::new(matrix.elem),
        );
        ExpandElement::Plain(variable)
    }

    // Try to return a reusable mutable variable for the given `item` or `None` otherwise.
    fn reuse_local_mut(&self, item: Item) -> Option<ExpandElement> {
        // Among the candidates, take a variable if it's only referenced by the pool.
        // Arbitrarily takes the first it finds in reversed order.
        self.local_mut_pool.borrow().get(&item).and_then(|vars| {
            vars.iter()
                .rev()
                .find(|var| matches!(var, ExpandElement::Managed(v) if Rc::strong_count(v) == 1))
                .cloned()
        })
    }

    /// Add a new variable to the pool with type specified by `item` for the given `scope`.
    pub fn add_local_mut(&self, item: Item) -> Rc<Variable> {
        let id = self.new_local_index();
        let local = Variable::new(VariableKind::LocalMut { id }, item);
        let var = Rc::new(local);
        let expand = ExpandElement::Managed(var.clone());
        let mut pool = self.local_mut_pool.borrow_mut();
        let variables = pool.entry(item).or_default();
        variables.push(expand);
        var
    }

    pub fn new_local_index(&self) -> u32 {
        self.next_id.fetch_add(1, Ordering::Release)
    }
}
