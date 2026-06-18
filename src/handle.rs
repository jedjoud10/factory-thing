use std::ops::Deref;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct PoleId(usize);

impl PoleId {
    pub const fn from_raw(index: usize) -> Self { Self(index) }
    pub const fn raw(self) -> usize { self.0 }
}

impl Deref for PoleId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for PoleId {
    fn from(value: usize) -> Self {
        Self::from_raw(value)
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub struct WireId(usize);

impl WireId {
    pub const fn from_raw(index: usize) -> Self { Self(index) }
    pub const fn raw(self) -> usize { self.0 }
}

impl Deref for WireId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<usize> for WireId {
    fn from(value: usize) -> Self {
        Self::from_raw(value)
    }
}