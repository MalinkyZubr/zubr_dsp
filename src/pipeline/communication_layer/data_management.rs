use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use std::mem::swap;


pub struct DataWrapper<T: Sharable + Send + Sync>{
    value: T
}

impl<T: Sharable + Send + Sync> DataWrapper<T>
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
