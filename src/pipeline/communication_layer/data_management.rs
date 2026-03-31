use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::mem::swap;


#[derive(Copy, Clone, Default)]
pub struct DataWrapper<T: Sharable + Send + Sync + Copy>{
    value: T
}

impl<T: Sharable + Send + Sync + Copy> DataWrapper<T>
where T: Copy {
    pub fn new() -> Self {
        DataWrapper {
            value: Default::default()
        }
    }
    
    pub fn new_with_value(value: T) -> Self {
        DataWrapper {
            value
        }
    }
    
    pub fn read(&mut self) -> &mut T {
        &mut self.value
    }
    
    pub fn swap(&mut self, other: &mut T) {
        swap(&mut self.value, other);
    }
    
    pub fn swap_st(&mut self, other: &mut Self) {
        swap(&mut self.value, &mut other.value);
    }
    
    pub fn copy_to_many<const N: usize>(&self, outputs: &mut [Self; N]) {
        for output in outputs.iter_mut() {
            output.value = self.value // bitwise copies
        }
    }
    
    pub fn copy_to(&self, output: &mut Self) {
        output.value = self.value
    }
}


#[derive(Copy, Clone)]
pub struct BufferArray<T: Sharable, const N: usize> {
    val: [T; N]
}
impl<T: Sharable, const N: usize> BufferArray<T, N> {
    pub fn new() -> Self {
        BufferArray {
            val: [Default::default(); N]
        }
    }
    
    pub fn new_with_value(value: [T; N]) -> Self {
        BufferArray {
            val: value
        }
    }
    
    pub fn read_mut(&mut self) -> &mut [T; N] {
        &mut self.val
    }
    
    pub fn read(&self) -> &[T; N] {
        &self.val
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

    // ---------- DataWrapper tests ----------

    #[test]
    fn test_new_default() {
        let wrapper: DataWrapper<i32> = DataWrapper::new();
        assert_eq!(*wrapper.clone().read(), 0);
    }

    #[test]
    fn test_new_with_value() {
        let mut wrapper = DataWrapper::new_with_value(42);
        assert_eq!(*wrapper.read(), 42);
    }

    #[test]
    fn test_read_mutability() {
        let mut wrapper = DataWrapper::new_with_value(10);
        *wrapper.read() = 20;
        assert_eq!(*wrapper.read(), 20);
    }

    #[test]
    fn test_swap_with_external() {
        let mut wrapper = DataWrapper::new_with_value(5);
        let mut external = 99;

        wrapper.swap(&mut external);

        assert_eq!(*wrapper.read(), 99);
        assert_eq!(external, 5);
    }

    #[test]
    fn test_swap_st_between_wrappers() {
        let mut a = DataWrapper::new_with_value(1);
        let mut b = DataWrapper::new_with_value(2);

        a.swap_st(&mut b);

        assert_eq!(*a.read(), 2);
        assert_eq!(*b.read(), 1);
    }

    #[test]
    fn test_copy_to() {
        let src = DataWrapper::new_with_value(7);
        let mut dst = DataWrapper::new();

        src.copy_to(&mut dst);

        assert_eq!(*dst.read(), 7);
    }

    #[test]
    fn test_copy_to_many() {
        let src = DataWrapper::new_with_value(11);
        let mut outputs = [DataWrapper::new(), DataWrapper::new(), DataWrapper::new()];

        src.copy_to_many(&mut outputs);

        for out in outputs.iter_mut() {
            assert_eq!(*out.read(), 11);
        }
    }

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