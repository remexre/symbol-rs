#![cfg_attr(not(feature = "std"), feature(alloc))]
#![cfg_attr(not(feature = "std"), no_std)]
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
//!
//! let s4 = Symbol::gensym();
//! assert_eq!(s4, "G#0");
//!
//! let s5: Symbol = "G#1".into();
//! assert_eq!(s5, "G#1");
//!
//! // symbol notices that G#1 is in use
//! let s6 = Symbol::gensym();
//! assert_eq!(s6, "G#2");
//! # }
//! ```

#[cfg(not(feature = "std"))]
extern crate alloc;
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
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

#[cfg(not(feature = "std"))]
use alloc::borrow::ToOwned;

#[cfg(not(feature = "std"))]
mod std {
    pub mod collections {
        pub use alloc::collections::BTreeSet;
    }
    pub mod cmp {
        pub use core::cmp::Ordering;
    }
    pub mod fmt {
        pub use core::fmt::{Debug, Display, Formatter, Result};
    }
    pub mod mem {
        pub use core::mem::{forget, transmute};
    }
    pub mod ops {
        pub use core::ops::Deref;
    }
}

use spin::Mutex;

lazy_static! {
    static ref SYMBOL_HEAP: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());
}

/// An interned string with O(1) equality.
#[allow(clippy::derive_hash_xor_eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(transparent))]
#[derive(Clone, Copy, Eq, Hash)]
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

    /// Generates a new symbol with a name of the form `G#n`, where `n` is some positive integer.
    pub fn gensym() -> Symbol {
        lazy_static! {
            static ref N: AtomicUsize = AtomicUsize::new(0);
        }

        let mut heap = SYMBOL_HEAP.lock();
        let n = loop {
            let n = leak_string(format!("G#{}", N.fetch_add(1, AtomicOrdering::SeqCst)));
            if heap.insert(n) {
                break n;
            }
        };
        drop(heap);

        Symbol::from(n)
    }

    /// A const fn that allows creating a [`Symbol`] from a `&'static str`
    ///
    /// ### Example:
    /// ```
    /// use symbol::Symbol;
    ///
    /// const MY_SYMBOL: Symbol = Symbol::from_static("this is a symbol");
    /// ```
    pub const fn from_static(lit: &'static str) -> Symbol {
        Symbol { s: lit }
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
                heap.insert(leak_string(s.to_owned()));
            }
        }
        let s = {
            let heap = SYMBOL_HEAP.lock();
            *heap.get(s).unwrap()
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

impl PartialOrd for Symbol {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
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

#[cfg(feature = "radix_trie")]
impl radix_trie::TrieKey for Symbol {
    fn encode_bytes(&self) -> Vec<u8> {
        self.as_str().encode_bytes()
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Symbol {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Symbol, D::Error> {
        String::deserialize(de).map(Symbol::from)
    }
}

fn leak_string(s: String) -> &'static str {
    let out = unsafe { transmute(&s as &str) };
    forget(s);
    out
}
