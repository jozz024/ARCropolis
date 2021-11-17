#[repr(C)]
#[derive(Debug, Copy, Clone)] // we are not making these, I am fine with allowing it to be copy
pub struct CppVector<T> {
    start: *mut T,
    end: *mut T,
    eos: *mut T,
}

// No reason to impl a `new` function, since we should only be analzying these when the game uses them.
impl<T> CppVector<T> {
    pub fn iter(&self) -> CppVectorIterator<T> {
        self.into_iter()
    }

    pub fn iter_mut(&mut self) -> CppVectorIteratorMut<T> {
        self.into_iter()
    }

    pub fn len(&self) -> usize {
        ((self.end as usize) - (self.start as usize)) / std::mem::size_of::<T>()
    }
}

impl<'a, T> IntoIterator for &'a CppVector<T> {
    type Item = &'a T;
    type IntoIter = CppVectorIterator<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        CppVectorIterator {
            vector: self,
            index: 0,
        }
    }
}

impl<'a, T> IntoIterator for &'a mut CppVector<T> {
    type Item = &'a mut T;
    type IntoIter = CppVectorIteratorMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        CppVectorIteratorMut {
            vector: self,
            index: 0,
        }
    }
}

pub struct CppVectorIterator<'a, T> {
    vector: &'a CppVector<T>,
    index: isize,
}

impl<'a, T> Iterator for CppVectorIterator<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        unsafe {
            if self.vector.start.offset(self.index) != self.vector.end {
                self.index += 1;
                Some(std::mem::transmute::<*mut T, &'a T>(
                    self.vector.start.offset(self.index - 1),
                ))
            } else {
                None
            }
        }
    }
}

pub struct CppVectorIteratorMut<'a, T> {
    vector: &'a mut CppVector<T>,
    index: isize,
}

impl<'a, T> Iterator for CppVectorIteratorMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        unsafe {
            if self.vector.start.offset(self.index) != self.vector.end {
                self.index += 1;
                Some(std::mem::transmute::<*mut T, &'a mut T>(
                    self.vector.start.offset(self.index - 1),
                ))
            } else {
                None
            }
        }
    }
}