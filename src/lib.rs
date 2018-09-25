//! Simple globally interned strings.
//!
//! # Usage
//!
//! ```
//! # extern crate symbol;
//! # use symbol::Symbol;
//! # fn main() {
//! let s1: Symbol = "asdf".into();
//! assert_eq!(s1, "asdf");
//!
//! let s2: Symbol = "asdf".into();
//! let s3: Symbol = "qwerty".into();
//!
//! assert_eq!(s1, s2);
//! assert_eq!(s1.addr(), s2.addr());
//!
//! assert_ne!(s2.addr(), s3.addr());
//! # }
//! ```

#[macro_use]
extern crate lazy_static;
extern crate spin;

#[cfg(feature = "gc")]
#[macro_use]
extern crate gc;

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt::{Debug, Display, Formatter, Result as FmtResult};
use std::mem::{forget, transmute};
use std::ops::Deref;

use spin::Mutex;

lazy_static! {
    static ref SYMBOL_HEAP: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
}

/// An interned string with O(1) equality.
#[derive(Clone, Copy, Eq, Hash, PartialOrd)]
pub struct Symbol {
    s: &'static str,
}

impl Symbol {
    /// Retrieves the address of the backing string.
    pub fn addr(self) -> usize {
        self.s.as_ptr() as usize
    }

    /// Retrieves the string from the Symbol.
    pub fn as_str(self) -> &'static str {
        self.s
    }
}

impl Debug for Symbol {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        Debug::fmt(self.s, fmt)
    }
}

impl Deref for Symbol {
    type Target = str;
    fn deref(&self) -> &str {
        self.s
    }
}

impl Display for Symbol {
    fn fmt(&self, fmt: &mut Formatter) -> FmtResult {
        fmt.write_str(self.s)
    }
}

impl<S: AsRef<str>> From<S> for Symbol {
    fn from(s: S) -> Symbol {
        let s = s.as_ref();
        {
            let mut heap = SYMBOL_HEAP.lock();
            if heap.get(s).is_none() {
                let string = s.to_owned();
                let s = unsafe { transmute(&string as &str) };
                forget(string);
                heap.insert(s);
            }
        }
        let s = {
            let heap = SYMBOL_HEAP.lock();
            heap.get(s).unwrap().clone()
        };
        Symbol { s }
    }
}

impl Ord for Symbol {
    fn cmp(&self, other: &Self) -> Ordering {
        let l = self.addr();
        let r = other.addr();
        l.cmp(&r)
    }
}

impl PartialEq for Symbol {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<S: AsRef<str>> PartialEq<S> for Symbol {
    fn eq(&self, other: &S) -> bool {
        self.partial_cmp(&other.as_ref()) == Some(Ordering::Equal)
    }
}

impl<S: AsRef<str>> PartialOrd<S> for Symbol {
    fn partial_cmp(&self, other: &S) -> Option<Ordering> {
        self.s.partial_cmp(other.as_ref())
    }
}

#[cfg(feature = "gc")]
impl ::gc::Finalize for Symbol {
    fn finalize(&self) {}
}

#[cfg(feature = "gc")]
unsafe impl ::gc::Trace for Symbol {
    unsafe_empty_trace!();
}
