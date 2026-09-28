//! Arena allocation of widget nodes.
//!
//! Owns the arena that holds every node, and the generational handle that stays
//! valid across insertions and deletions.

/// A generational handle to a value in an [`Arena`].
///
/// A handle pairs the index of the slot holding the value with the generation
/// that slot was allocated in. Slots never move, so the index stays stable
/// across growth and across other values being removed; the generation is what
/// keeps an old handle honest — [`Arena::remove`] bumps it, so a handle from
/// before the removal stops resolving, and a value that reuses the freed slot
/// gets a handle the old one cannot be mistaken for.
///
/// Handles are `Copy` values; the arena is the only thing that can create one.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Handle {
    index: u32,
    generation: u32,
}

/// A single slot: the value it currently holds (`None` while the slot is on the
/// free list) and the generation its current occupant was allocated in.
struct Slot<T> {
    value: Option<T>,
    generation: u32,
}

/// Converts a handle index to a slot position.
///
/// Indices the arena issues are `Vec` lengths, so they fit `usize` on every
/// target this arena runs on; the fallback exists because `TryFrom` cannot
/// know that, and it lands on a bounds-checked `None`.
fn slot_position(index: u32) -> usize {
    usize::try_from(index).unwrap_or(usize::MAX)
}

/// A generational arena with stable handles and O(1) insert and remove.
///
/// Freed slots go on a free list and are reused by the next insertion, so
/// insert/remove churn does not grow the backing storage. Iteration walks the
/// slots in index order and skips the free ones.
///
/// ```
/// use ui_core::arena::Arena;
///
/// let mut arena = Arena::new();
/// let handle = arena.insert(7);
/// assert_eq!(arena.get(handle), Some(&7));
///
/// assert_eq!(arena.remove(handle), Some(7));
/// assert_eq!(arena.get(handle), None);
/// ```
pub struct Arena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<usize>,
    len: usize,
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Arena<T> {
    /// Creates an empty arena.
    pub fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Inserts `value` into the arena and returns the handle that resolves to it.
    ///
    /// Reuses the most recently freed slot when there is one, so the slot count
    /// tracks live entries plus churn rather than total insertions.
    #[must_use]
    pub fn insert(&mut self, value: T) -> Handle {
        let index = match self.free.pop() {
            Some(index) => index,
            None => {
                let index = self.slots.len();
                self.slots.push(Slot {
                    value: None,
                    generation: 0,
                });
                index
            }
        };
        let slot = &mut self.slots[index];
        slot.value = Some(value);
        self.len += 1;
        Handle {
            // Saturate rather than truncate: an index that does not fit `u32`
            // cannot be represented in a `Handle`, and truncating would alias
            // a different slot. Unreachable in practice — it needs more than
            // 4 billion slots.
            index: u32::try_from(index).unwrap_or(u32::MAX),
            generation: slot.generation,
        }
    }

    /// Returns a reference to the value `handle` points to, or `None` if the
    /// handle is stale or was never valid.
    pub fn get(&self, handle: Handle) -> Option<&T> {
        let slot = self.slots.get(slot_position(handle.index))?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_ref()
    }

    /// Returns a mutable reference to the value `handle` points to, or `None`
    /// if the handle is stale or was never valid.
    pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
        let slot = self.slots.get_mut(slot_position(handle.index))?;
        if slot.generation != handle.generation {
            return None;
        }
        slot.value.as_mut()
    }

    /// Removes the value `handle` points to and returns it, or `None` if the
    /// handle is stale or was never valid.
    ///
    /// The slot's generation is bumped and the slot returns to the free list,
    /// so this handle stops resolving and the next insertion into that slot
    /// gets a fresh handle.
    pub fn remove(&mut self, handle: Handle) -> Option<T> {
        let index = slot_position(handle.index);
        let slot = self.slots.get_mut(index)?;
        if slot.generation != handle.generation {
            return None;
        }
        let value = slot.value.take()?;
        // Wrapping is deliberate: a slot would need 4 billion remove cycles to
        // wrap, and a handle from before the wrap would have to survive every
        // one of them to be wrongly accepted.
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(index);
        self.len -= 1;
        Some(value)
    }

    /// Iterates the live values in slot order, skipping removed entries.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().filter_map(|slot| slot.value.as_ref())
    }

    /// Returns the number of live values in the arena.
    #[must_use]
    // The task fixes the arena's query surface at `len`; `is_empty` is
    // deliberately not part of it.
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if `handle` currently resolves to a live value.
    #[must_use]
    pub fn is_valid(&self, handle: Handle) -> bool {
        self.get(handle).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_retrieve() {
        let mut arena = Arena::new();
        assert_eq!(arena.len(), 0);
        assert_eq!(arena.iter().count(), 0);

        let root = arena.insert("root");
        assert_eq!(arena.len(), 1);
        assert!(arena.is_valid(root));
        assert_eq!(arena.get(root), Some(&"root"));
        assert_eq!(arena.iter().collect::<Vec<_>>(), vec![&"root"]);

        let child = arena.insert("child");
        assert_eq!(arena.len(), 2);
        assert_eq!(arena.get(child), Some(&"child"));

        *arena.get_mut(root).unwrap() = "renamed";
        assert_eq!(arena.get(root), Some(&"renamed"));
        assert_eq!(arena.get(child), Some(&"child"));
    }

    #[test]
    fn remove_invalidates_handle() {
        let mut arena = Arena::new();
        let handle = arena.insert(1);

        assert_eq!(arena.remove(handle), Some(1));
        assert_eq!(arena.len(), 0);
        assert!(!arena.is_valid(handle));
        assert_eq!(arena.get(handle), None);
        assert_eq!(arena.get_mut(handle), None);
        assert_eq!(arena.remove(handle), None);
        assert_eq!(arena.iter().count(), 0);
    }

    #[test]
    fn reuse_bumps_generation() {
        let mut arena = Arena::new();
        let first = arena.insert("a");
        let second = arena.insert("b");
        assert_eq!(arena.remove(first), Some("a"));

        let reused = arena.insert("c");
        assert_ne!(first, reused);
        assert!(!arena.is_valid(first));
        assert!(arena.is_valid(reused));
        assert_eq!(arena.get(reused), Some(&"c"));
        assert_eq!(arena.get(second), Some(&"b"));
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn iteration_skips_removed() {
        let mut arena = Arena::new();
        let a = arena.insert(1);
        let b = arena.insert(2);
        let c = arena.insert(3);
        assert_eq!(arena.remove(b), Some(2));

        let mut values = arena.iter().copied();
        assert_eq!(values.next(), Some(1));
        assert_eq!(values.next(), Some(3));
        assert_eq!(values.next(), None);
        assert_eq!(arena.len(), 2);
        assert!(arena.is_valid(a));
        assert!(!arena.is_valid(b));
        assert!(arena.is_valid(c));
    }

    #[test]
    fn growth_preserves_handles() {
        let mut arena = Arena::new();
        let mut handles = Vec::new();
        for i in 0..1000 {
            handles.push(arena.insert(i));
        }

        // Churn past the growth the loop above already forced.
        for _ in 0..100 {
            let handle = arena.insert(0);
            arena.remove(handle);
        }

        for (i, handle) in handles.iter().enumerate() {
            assert!(arena.is_valid(*handle));
            assert_eq!(arena.get(*handle), Some(&i));
        }
        assert_eq!(arena.len(), 1000);
    }
}
