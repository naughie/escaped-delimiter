//! `escaped-delimiter` provides an iterator of a delimited slice, considering an escape character.
//!
//! See the examples below.
//!
//! # Examples
//!
//! ```
//! use escaped_delimiter::iter;
//!
//! // Without escape characters
//! let s = b"The quick brown fox";
//! let s_vec: Vec<_> = iter(s, b' ', b'\\').collect();
//! assert_eq!(s_vec, &[&b"The"[..], &b"quick"[..], &b"brown"[..], &b"fox"[..]]);
//!
//! // Reverse it (`DoubleEndedIterator`)
//! let s = b"The quick brown fox";
//! let s_vec: Vec<_> = iter(s, b' ', b'\\').rev().collect();
//! assert_eq!(s_vec, &[&b"fox"[..], &b"brown"[..], &b"quick"[..], &b"The"[..]]);
//!
//! // With escape characters
//! let s = b"a\\ b\\\\ c\\\\\\ d\\\\\\\\ e";
//! let s_vec: Vec<_> = iter(s, b' ', b'\\').collect();
//! assert_eq!(s_vec, &[&b"a\\ b\\\\"[..], &b"c\\\\\\ d\\\\\\\\"[..], &b"e"[..]]);
//! ```

use std::num::NonZeroUsize;

pub fn iter(slice: &[u8], delim: u8, escape: u8) -> Iter<'_> {
    Iter::from_slice(slice, delim, escape)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Iter<'a> {
    delim: u8,
    escape: u8,
    inner: &'a [u8],
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<Self::Item> {
        let pos = self.find_bow();
        self.inner = &self.inner[pos..];

        let pos = self.find_eow()?.get();
        let inner = &self.inner[..pos];
        self.inner = &self.inner[pos..];

        Some(inner)
    }
}

impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let pos = self.rfind_eow()?.get();
        self.inner = &self.inner[..pos];

        let pos = self.rfind_bow();
        let inner = &self.inner[pos..];
        self.inner = &self.inner[..pos];

        Some(inner)
    }
}

impl Iter<'_> {
    #[inline]
    fn len(self) -> usize {
        self.inner.len()
    }

    #[inline]
    fn is_empty(self) -> bool {
        self.inner.is_empty()
    }

    fn enumerate(&self) -> impl DoubleEndedIterator<Item = (usize, u8)> + '_ {
        self.inner.iter().copied().enumerate()
    }

    #[inline]
    fn renumerate(&self) -> impl Iterator<Item = (usize, u8)> + '_ {
        self.enumerate().rev()
    }

    fn find_bow(&self) -> usize {
        let mut it = self.enumerate().skip_while(|&(_, c)| c == self.delim);

        if let Some((i, _)) = it.next() {
            i
        } else {
            self.len()
        }
    }

    fn find_eow(&self) -> Option<NonZeroUsize> {
        if self.is_empty() {
            return None;
        }

        let mut prev_char = 0u8;
        for (i, c) in self.enumerate() {
            if c == self.delim && prev_char != self.escape {
                // SAFETY: self.inner[0] != DELIM
                return unsafe { Some(NonZeroUsize::new_unchecked(i)) };
            }

            prev_char = if c == self.escape && prev_char == self.escape {
                0
            } else {
                c
            };
        }

        // SAFETY: self.inner.len() > 0
        unsafe { Some(NonZeroUsize::new_unchecked(self.len())) }
    }

    fn rfind_eow(&self) -> Option<NonZeroUsize> {
        let mut it = self.renumerate().skip_while(|&(_, c)| c == self.delim);

        if let Some((i, c)) = it.next() {
            if c == self.escape {
                // [^ESCAPE] ESCAPE ESCAPE* ESCAPE \t*
                //             ^              ^
                //             j              i
                let last = match it.filter(|&(_, c)| c == self.escape).last() {
                    Some((j, _)) if !iso_parity(i, j) => i + 1,
                    _ => i + 2,
                };
                // SAFETY: i + 1 > 0
                unsafe { Some(NonZeroUsize::new_unchecked(last)) }
            } else {
                // SAFETY: i + 1 > 0
                unsafe { Some(NonZeroUsize::new_unchecked(i + 1)) }
            }
        } else {
            None
        }
    }

    fn rfind_bow(&self) -> usize {
        let mut delim = 0;
        let mut delim_found = false;
        let mut broken = false;

        for (i, c) in self.renumerate() {
            if delim_found && c != self.escape {
                // [^ESCAPE] ESCAPE* DELIM
                //     ^               ^
                //     i             delim
                if iso_parity(i, delim) {
                    // # of ESCAPE's is odd
                    delim_found = false;
                } else {
                    // # of ESCAPE's is even
                    broken = true;
                    break;
                }
            }
            if c == self.delim {
                delim_found = true;
                delim = i;
            }
        }

        if delim_found && (broken || iso_parity(delim, 0)) {
            delim + 1
        } else {
            0
        }
    }
}

#[inline]
fn iso_parity(i: usize, j: usize) -> bool {
    (i & 1) == (j & 1)
}

impl<'a> Iter<'a> {
    /// Returns the rest of the inner slice.
    ///
    /// ```
    /// use escaped_delimiter::iter;
    ///
    /// let s = b"abc";
    /// let it = iter(s, b' ', b'\\');
    /// assert_eq!(it.as_slice(), &b"abc"[..]);
    ///
    /// let s = b"a b c d";
    /// let mut it = iter(s, b' ', b'\\');
    /// it.next(); // consumes b'a'
    /// it.next_back(); // consumes b'd'
    /// assert_eq!(it.as_slice(), &b" b c "[..]);
    /// ```
    #[inline]
    pub fn as_slice(&self) -> &'a [u8] {
        self.inner
    }

    /// See the examples at the top of this doc page.
    #[inline]
    pub fn from_slice(inner: &'a [u8], delim: u8, escape: u8) -> Self {
        Self {
            inner,
            delim,
            escape,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_words() {
        let s = b"";
        let mut words = Iter::from_slice(s, b'X', b'Y');
        assert_eq!(words.next(), None);

        let s = b"abc";
        let mut words = Iter::from_slice(s, b'X', b'Y');
        assert_eq!(words.next(), Some(&b"abc"[..]));
        assert_eq!(words.next(), None);

        let s = b"abcX";
        let mut words = Iter::from_slice(s, b'X', b'Y');
        assert_eq!(words.next(), Some(&b"abc"[..]));
        assert_eq!(words.next(), None);

        let s = b"abcXdefXXhX jklm";
        let mut words = Iter::from_slice(s, b'X', b'Y');
        assert_eq!(words.next(), Some(&b"abc"[..]));
        assert_eq!(words.next(), Some(&b"def"[..]));
        assert_eq!(words.next(), Some(&b"h"[..]));
        assert_eq!(words.next(), Some(&b" jklm"[..]));
        assert_eq!(words.next(), None);

        let s = b"abXYXcdeXYfYXXYYYXgYYX";
        let mut words = Iter::from_slice(s, b'X', b'Y');
        assert_eq!(words.next(), Some(&b"ab"[..]));
        assert_eq!(words.next(), Some(&b"YXcde"[..]));
        assert_eq!(words.next(), Some(&b"YfYX"[..]));
        assert_eq!(words.next(), Some(&b"YYYXgYY"[..]));
        assert_eq!(words.next(), None);
    }

    #[test]
    fn test_words_rev() {
        let s = b"";
        let mut words = Iter::from_slice(s, b'X', b'Y').rev();
        assert_eq!(words.next(), None);

        let s = b"abcXdefXXhX jklm";
        let mut words = Iter::from_slice(s, b'X', b'Y').rev();
        assert_eq!(words.next(), Some(&b" jklm"[..]));
        assert_eq!(words.next(), Some(&b"h"[..]));
        assert_eq!(words.next(), Some(&b"def"[..]));
        assert_eq!(words.next(), Some(&b"abc"[..]));
        assert_eq!(words.next(), None);

        let s = b"XXabXYXcdeXYfYXXYYYXgYYX";
        let mut words = Iter::from_slice(s, b'X', b'Y').rev();
        assert_eq!(words.next(), Some(&b"YYYXgYY"[..]));
        assert_eq!(words.next(), Some(&b"YfYX"[..]));
        assert_eq!(words.next(), Some(&b"YXcde"[..]));
        assert_eq!(words.next(), Some(&b"ab"[..]));
        assert_eq!(words.next(), None);

        let s = b"Xa";
        let mut words = Iter::from_slice(s, b'X', b'Y').rev();
        assert_eq!(words.next(), Some(&b"a"[..]));
        assert_eq!(words.next(), None);

        let s = b"YXa";
        let mut words = Iter::from_slice(s, b'X', b'Y').rev();
        assert_eq!(words.next(), Some(&b"YXa"[..]));
        assert_eq!(words.next(), None);

        let s = b"YYXa";
        let mut words = Iter::from_slice(s, b'X', b'Y').rev();
        assert_eq!(words.next(), Some(&b"a"[..]));
        assert_eq!(words.next(), Some(&b"YY"[..]));
        assert_eq!(words.next(), None);
    }

    #[test]
    fn test_words_mixed() {
        let s = b"abcXdefXXhX jklm";
        let mut words = Iter::from_slice(s, b'X', b'Y');
        assert_eq!(words.next(), Some(&b"abc"[..]));
        assert_eq!(words.next_back(), Some(&b" jklm"[..]));
        assert_eq!(words.next(), Some(&b"def"[..]));
        assert_eq!(words.next_back(), Some(&b"h"[..]));
        assert_eq!(words.next(), None);
        assert_eq!(words.next_back(), None);
    }
}
