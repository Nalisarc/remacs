use remacs_macros::lisp_fn;
use libc::c_void;
use lisp::{LispObject, ExternalPtr};
use remacs_sys::{Lisp_Hash_Table, PseudovecType, Fcopy_sequence, Faref, hash_lookup, EmacsInt,
                 EmacsUint, CHECK_IMPURE, hash_remove_from_table, gc_aset, hash_put};
use std::ptr;

pub type LispHashTableRef = ExternalPtr<Lisp_Hash_Table>;

impl LispHashTableRef {
    pub fn allocate() -> LispHashTableRef {
        let vec_ptr =
            allocate_pseudovector!(Lisp_Hash_Table, count, PseudovecType::PVEC_HASH_TABLE);
        LispHashTableRef::new(vec_ptr)
    }

    pub unsafe fn copy(&mut self, other: LispHashTableRef) {
        ptr::copy_nonoverlapping(other.as_ptr(), self.as_mut(), 1);
    }

    pub fn set_next_weak(&mut self, other: LispHashTableRef) {
        self.next_weak = other.as_ptr() as *mut Lisp_Hash_Table;
    }

    pub fn get_next_weak(&self) -> LispHashTableRef {
        LispHashTableRef::new(self.next_weak)
    }

    pub fn set_hash(&mut self, hash: LispObject) {
        self.hash = hash.to_raw();
    }

    pub fn get_hash(&self) -> LispObject {
        LispObject::from(self.hash)
    }

    pub fn set_next(&mut self, next: LispObject) {
        self.next = next.to_raw();
    }

    pub fn get_next(&self) -> LispObject {
        LispObject::from(self.next)
    }

    pub fn set_index(&mut self, index: LispObject) {
        self.index = index.to_raw();
    }

    pub fn get_index(&self) -> LispObject {
        LispObject::from(self.index)
    }

    pub fn get_key_and_value(&self) -> LispObject {
        LispObject::from(self.key_and_value)
    }

    pub fn set_key_and_value(&mut self, key_and_value: LispObject) {
        self.key_and_value = key_and_value.to_raw();
    }

    pub fn get_weak(&self) -> LispObject {
        LispObject::from(self.weak)
    }

    #[inline]
    pub fn get_hash_value(self, idx: isize) -> LispObject {
        let index = LispObject::from_natnum((2 * idx + 1) as EmacsInt);
        unsafe { LispObject::from(Faref(self.key_and_value, index.to_raw())) }
    }

    #[inline]
    pub fn set_hash_value(self, idx: isize, value: LispObject) {
        unsafe { gc_aset(self.key_and_value, 2 * idx + 1, value.to_raw()) };
    }

    pub fn lookup(self, key: LispObject, hashptr: *mut EmacsUint) -> isize {
        let mutself = self.as_ptr() as *mut Lisp_Hash_Table;
        unsafe { hash_lookup(mutself, key.to_raw(), hashptr) }
    }

    pub fn put(mut self, key: LispObject, value: LispObject, hash: EmacsUint) -> isize {
        unsafe { hash_put(self.as_mut(), key.to_raw(), value.to_raw(), hash) }
    }

    pub fn check_impure(self, object: LispObject) {
        unsafe { CHECK_IMPURE(object.to_raw(), self.as_ptr() as *mut c_void) };
    }

    pub fn remove(mut self, key: LispObject) {
        unsafe { hash_remove_from_table(self.as_mut(), key.to_raw()) };
    }

    pub fn get_hash_hash(self, idx: isize) -> LispObject {
        let index = LispObject::from_natnum(idx as EmacsInt);
        unsafe { LispObject::from_raw(Faref(self.hash, index.to_raw())) }
    }

    pub fn get_hash_key(self, idx: isize) -> LispObject {
        let index = LispObject::from_natnum((2 * idx) as EmacsInt);
        unsafe { LispObject::from_raw(Faref(self.key_and_value, index.to_raw())) }
    }
}

/// An iterator used for iterating over the indices
/// of the 'key_and_value' vector of a Lisp_Hash_Table.
/// Equivalent to a 'for (i = 0; i < HASH_TABLE_SIZE(h); ++i)'
/// loop in the C layer.
pub struct HashTableIter<'a> {
    table: &'a LispHashTableRef,
    current: usize,
}

impl<'a> Iterator for HashTableIter<'a> {
    type Item = isize;

    fn next(&mut self) -> Option<isize> {
        let next_vector = unsafe { self.table.get_next().as_vector_unchecked() };
        if self.current < next_vector.len() {
            let cur = self.current;
            self.current += 1;
            Some(cur as isize)
        } else {
            None
        }
    }
}

/// An iterator used for looping over the keys and values
/// contained in a Lisp_Hash_Table.
pub struct KeyAndValueIter<'a>(HashTableIter<'a>);

impl<'a> Iterator for KeyAndValueIter<'a> {
    type Item = (LispObject, LispObject);

    fn next(&mut self) -> Option<(LispObject, LispObject)> {
        while let Some(idx) = self.0.next() {
            let is_not_nil = self.0.table.get_hash_hash(idx).is_not_nil();
            if is_not_nil {
                let key = self.0.table.get_hash_key(idx);
                let value = self.0.table.get_hash_value(idx);
                return Some((key, value));
            }
        }

        None
    }
}

impl LispHashTableRef {
    pub fn indices(&self) -> HashTableIter {
        HashTableIter {
            table: self,
            current: 0,
        }
    }

    pub fn iter(&self) -> KeyAndValueIter {
        KeyAndValueIter(self.indices())
    }
}

/// Return a copy of hash table TABLE.
/// Keys and values are not copied, only the table itself is.
#[lisp_fn]
fn copy_hash_table(htable: LispObject) -> LispObject {
    let mut table = htable.as_hash_table_or_error();
    let mut new_table = LispHashTableRef::allocate();
    unsafe { new_table.copy(table) };
    assert_ne!(new_table.as_ptr(), table.as_ptr());

    let key_and_value = LispObject::from(unsafe {
        Fcopy_sequence(new_table.get_key_and_value().to_raw())
    });
    let hash = LispObject::from(unsafe { Fcopy_sequence(new_table.get_hash().to_raw()) });
    let next = LispObject::from(unsafe { Fcopy_sequence(new_table.get_next().to_raw()) });
    let index = LispObject::from(unsafe { Fcopy_sequence(new_table.get_index().to_raw()) });
    new_table.set_key_and_value(key_and_value);
    new_table.set_hash(hash);
    new_table.set_next(next);
    new_table.set_index(index);

    if new_table.get_weak().is_not_nil() {
        new_table.set_next_weak(table.get_next_weak());
        table.set_next_weak(new_table);
    }

    LispObject::from_hash_table(new_table)
}

/// Look up KEY in TABLE and return its associated value.
/// If KEY is not found, return DFLT which defaults to nil.
#[lisp_fn(min = "2")]
fn gethash(key: LispObject, table: LispObject, dflt: LispObject) -> LispObject {
    let hash_table = table.as_hash_table_or_error();
    let idx = hash_table.lookup(key, ptr::null_mut());

    if idx >= 0 {
        hash_table.get_hash_value(idx)
    } else {
        dflt
    }
}

/// Associate KEY with VALUE in hash table TABLE.
/// If KEY is already present in table, replace its current value with
/// VALUE.  In any case, return VALUE.
#[lisp_fn]
fn puthash(key: LispObject, value: LispObject, table: LispObject) -> LispObject {
    let hash_table = table.as_hash_table_or_error();
    hash_table.check_impure(table);

    let mut hash: EmacsUint = 0;
    let idx = hash_table.lookup(key, &mut hash);

    if idx >= 0 {
        hash_table.set_hash_value(idx, value);
    } else {
        hash_table.put(key, value, hash);
    }

    value
}

/// Remove KEY from TABLE.
#[lisp_fn]
fn remhash(key: LispObject, table: LispObject) -> LispObject {
    let hash_table = table.as_hash_table_or_error();
    hash_table.check_impure(table);
    hash_table.remove(key);

    LispObject::constant_nil()
}

/// Call FUNCTION for all entries in hash table TABLE.
/// FUNCTION is called with two arguments, KEY and VALUE.
/// `maphash' always returns nil.
#[lisp_fn]
fn maphash(function: LispObject, table: LispObject) -> LispObject {
    let hash_table = table.as_hash_table_or_error();
    for (key, value) in hash_table.iter() {
        call!(function, key, value);
    }

    LispObject::constant_nil()
}

/// Return t if OBJ is a Lisp hash table object.
#[lisp_fn]
fn hash_table_p(obj: LispObject) -> LispObject {
    LispObject::from_bool(obj.is_hash_table())
}

/// Return the number of elements in TABLE.
#[lisp_fn]
fn hash_table_count(table: LispObject) -> LispObject {
    LispObject::from_natnum(table.as_hash_table_or_error().count as EmacsInt)
}
