use std::mem;
use crate::engine::data_plane::structural::pipeline_type_traits::{Sharable};
use std::mem::swap;


#[derive(Clone)]
pub struct BufferArray<T: Sharable, const N: usize> {
    val: Box<[T; N]>,
}
impl<T: Sharable, const N: usize> BufferArray<T, N> {
    pub fn swap_pointers(&mut self, other: &mut BufferArray<T, N>) {
        swap(&mut self.val, &mut other.val);
    }
    
    pub fn new() -> Self {
        BufferArray {
            val: Box::new(std::array::from_fn(|_| Default::default())),
        }
    }
    pub fn copy_to(&self, output: &mut Self) {
        output.val.clone_from(&self.val);
    }
    pub fn new_with_value(value: [T; N]) -> Self {
        BufferArray { val: Box::new(value) }
    }

    pub fn read_mut(&mut self) -> &mut [T; N] {
        &mut self.val
    }

    pub fn read(&self) -> &[T; N] {
        &self.val
    }

    pub fn get_mut(&mut self, index: usize) -> &mut T {
        &mut self.val[index]
    }

    pub fn get(&self, index: usize) -> &T {
        &self.val[index]
    }

    pub fn set(&mut self, index: usize, value: T) {
        self.val[index] = value;
    }

    pub fn len(&self) -> usize {
        N
    }
    
    pub fn mutate(&mut self, index: usize, mut f: impl FnMut(&mut T)) {
        f(&mut self.val[index]);
    }
    
    pub fn mutate_all(&mut self, f: impl FnMut(&mut T)) {
        self.val.iter_mut().for_each(f);
    }

    pub fn reverse(&mut self) {
        self.val.reverse();
    }
}
impl<T: Sharable, const N: usize> Default for BufferArray<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- BufferArray tests ----------

    #[test]
    fn test_buffer_array_new_default() {
        let buf: BufferArray<i32, 4> = BufferArray::new();
        assert_eq!(buf.read(), &[0, 0, 0, 0]);
    }

    #[test]
    fn test_buffer_array_read_mut() {
        let mut buf: BufferArray<i32, 3> = BufferArray::new();
        let data = buf.read_mut();

        data[0] = 1;
        data[1] = 2;
        data[2] = 3;

        assert_eq!(buf.read(), &[1, 2, 3]);
    }

    #[test]
    fn test_buffer_array_read() {
        let mut buf: BufferArray<i32, 2> = BufferArray::new();
        buf.read_mut()[0] = 10;
        buf.read_mut()[1] = 20;

        let read_ref = buf.read();
        assert_eq!(read_ref, &[10, 20]);
    }

    #[test]
    fn test_buffer_array_default_trait() {
        let buf: BufferArray<i32, 5> = Default::default();
        assert_eq!(buf.read(), &[0, 0, 0, 0, 0]);
    }
}
