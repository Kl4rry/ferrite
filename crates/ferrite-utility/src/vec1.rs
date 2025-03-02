use std::{
    fmt::{self, Debug},
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

use serde::{Deserialize, Serialize, ser::SerializeSeq as _};

#[derive(Clone, PartialEq, Eq)]
pub struct Vec1<T> {
    inner: Vec<T>,
}

impl<T> Debug for Vec1<T>
where
    T: Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl<T> Vec1<T> {
    pub fn new(first: T) -> Self {
        Self { inner: vec![first] }
    }

    pub fn from_vec(vec: Vec<T>) -> Option<Self> {
        if vec.is_empty() {
            None
        } else {
            Some(Self { inner: vec })
        }
    }

    pub fn clear(&mut self) {
        self.inner.drain(1..);
    }

    pub fn push(&mut self, value: T) {
        self.inner.push(value);
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.inner.len() == 1 {
            return None;
        }
        self.inner.pop()
    }

    pub fn first(&self) -> &T {
        unsafe { self.inner.get_unchecked(0) }
    }

    pub fn first_mut(&mut self) -> &mut T {
        unsafe { self.inner.get_unchecked_mut(0) }
    }

    pub fn end(&self) -> &T {
        unsafe { self.inner.get_unchecked(self.inner.len() - 1) }
    }

    pub fn end_mut(&mut self) -> &mut T {
        let len = self.inner.len();
        unsafe { self.inner.get_unchecked_mut(len - 1) }
    }

    pub fn remove(&mut self, index: usize) -> Option<T> {
        if index == 0 {
            return None;
        }
        if index < self.inner.len() {
            return Some(self.inner.remove(index));
        }
        None
    }

    pub fn replace_with_slice(&mut self, slice: &[T])
    where
        T: Clone,
    {
        if !slice.is_empty() {
            self.inner.clear();
            self.inner.extend_from_slice(slice);
        }
    }
}

impl<T> Default for Vec1<T>
where
    T: Default,
{
    fn default() -> Self {
        Self {
            inner: vec![T::default()],
        }
    }
}

impl<T> Deref for Vec1<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> DerefMut for Vec1<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<T> serde::Serialize for Vec1<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for e in &self.inner {
            seq.serialize_element(e)?;
        }
        seq.end()
    }
}

struct Vec1Visitor<T> {
    marker: PhantomData<fn() -> Vec1<T>>,
}

impl<T> Vec1Visitor<T> {
    fn new() -> Self {
        Vec1Visitor {
            marker: PhantomData,
        }
    }
}

impl<'de, T> serde::de::Visitor<'de> for Vec1Visitor<T>
where
    T: Deserialize<'de> + Default,
{
    type Value = Vec1<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("an sequence with atleast len 1")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut vec: Vec<T> = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(v) = seq.next_element()? {
            vec.push(v);
        }

        Ok(Vec1::from_vec(vec).unwrap_or_default())
    }
}

impl<'de, T> Deserialize<'de> for Vec1<T>
where
    T: Deserialize<'de> + Default,
{
    fn deserialize<D>(deserializer: D) -> Result<Vec1<T>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_seq(Vec1Visitor::new())
    }
}
