use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;
use std::iter::Sum;

pub struct ConstAdder<T: Copy, const BUFFER_SIZE: usize> {
    pub const_values: [T; BUFFER_SIZE],
}
impl<T: Copy, const BUFFER_SIZE: usize> ConstAdder<T, BUFFER_SIZE> {
    pub fn new(const_values: [T; BUFFER_SIZE]) -> Self {
        Self { const_values }
    }
}
impl<T: Sharable + Num + Sum + std::ops::AddAssign<T> + Copy, const BUFFER_SIZE: usize>
    PipelineStep<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, 1>
    for ConstAdder<T, BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BUFFER_SIZE>>; 1],
        output: &mut DataWrapper<BufferArray<T, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            *output.read().get_mut(idx) = *input[0].read().get(idx);
            *output.read().get_mut(idx) += self.const_values[idx];
        }
        Ok(())
    }
}

pub struct ConstSubtractor<T: Copy, const BUFFER_SIZE: usize> {
    pub const_values: [T; BUFFER_SIZE],
}
impl<T: Copy, const BUFFER_SIZE: usize> ConstSubtractor<T, BUFFER_SIZE> {
    pub fn new(const_values: [T; BUFFER_SIZE]) -> Self {
        Self { const_values }
    }
}
impl<T: Sharable + Num + std::ops::SubAssign<T> + Copy, const BUFFER_SIZE: usize>
    PipelineStep<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, 1>
    for ConstSubtractor<T, BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BUFFER_SIZE>>; 1],
        output: &mut DataWrapper<BufferArray<T, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            *output.read().get_mut(idx) = *input[0].read().get(idx);
            *output.read().get_mut(idx) -= self.const_values[idx];
        }
        Ok(())
    }
}

pub struct ConstMultiplier<T: Copy, const BUFFER_SIZE: usize> {
    pub const_values: [T; BUFFER_SIZE],
}
impl<T: Copy, const BUFFER_SIZE: usize> ConstMultiplier<T, BUFFER_SIZE> {
    pub fn new(const_values: [T; BUFFER_SIZE]) -> Self {
        Self { const_values }
    }
}
impl<T: Sharable + Num + std::ops::MulAssign<T> + Copy, const BUFFER_SIZE: usize>
    PipelineStep<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, 1>
    for ConstMultiplier<T, BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BUFFER_SIZE>>; 1],
        output: &mut DataWrapper<BufferArray<T, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            *output.read().get_mut(idx) = *input[0].read().get(idx);
            *output.read().get_mut(idx) *= self.const_values[idx];
        }
        Ok(())
    }
}

pub struct ConstDivider<T: Copy, const BUFFER_SIZE: usize> {
    pub const_values: [T; BUFFER_SIZE],
}
impl<T: Copy, const BUFFER_SIZE: usize> ConstDivider<T, BUFFER_SIZE> {
    pub fn new(const_values: [T; BUFFER_SIZE]) -> Self {
        Self { const_values }
    }
}
impl<T: Sharable + Num + std::ops::DivAssign<T> + Copy, const BUFFER_SIZE: usize>
    PipelineStep<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, 1>
    for ConstDivider<T, BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BUFFER_SIZE>>; 1],
        output: &mut DataWrapper<BufferArray<T, BUFFER_SIZE>>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            *output.read().get_mut(idx) = *input[0].read().get(idx);
            *output.read().get_mut(idx) /= self.const_values[idx];
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped_buffer<const N: usize>(values: [i32; N]) -> DataWrapper<BufferArray<i32, N>> {
        DataWrapper::new_with_value(BufferArray::new_with_value(values))
    }

    #[test]
    fn const_adder_adds_constant_values() {
        let mut step = ConstAdder::<i32, 4>::new([10, 20, 30, 40]);
        let mut input = [wrapped_buffer([1, 2, 3, 4])];
        let mut output = DataWrapper::new_with_value(BufferArray::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[11, 22, 33, 44]);
    }

    #[test]
    fn const_subtractor_subtracts_constant_values() {
        let mut step = ConstSubtractor::<i32, 4>::new([1, 2, 3, 4]);
        let mut input = [wrapped_buffer([10, 20, 30, 40])];
        let mut output = DataWrapper::new_with_value(BufferArray::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[9, 18, 27, 36]);
    }

    #[test]
    fn const_multiplier_multiplies_by_constant_values() {
        let mut step = ConstMultiplier::<i32, 4>::new([2, 3, 4, 5]);
        let mut input = [wrapped_buffer([1, 2, 3, 4])];
        let mut output = DataWrapper::new_with_value(BufferArray::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[2, 6, 12, 20]);
    }

    #[test]
    fn const_divider_divides_by_constant_values() {
        let mut step = ConstDivider::<i32, 4>::new([2, 4, 3, 8]);
        let mut input = [wrapped_buffer([10, 20, 9, 32])];
        let mut output = DataWrapper::new_with_value(BufferArray::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[5, 5, 3, 4]);
    }
}