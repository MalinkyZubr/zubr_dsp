use crate::engine::data_plane::communication_layer::data_management::{BufferArray};
use crate::engine::data_plane::structural::generic_node_operation::PipelineNodeOp;
use crate::engine::data_plane::structural::pipeline_type_traits::Sharable;
use num::Num;
use std::iter::Sum;

pub struct PointwiseAdder<const BUFFER_SIZE: usize> {}
impl<const BUFFER_SIZE: usize> PointwiseAdder<BUFFER_SIZE> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<
        T: Sharable + Num + Sum + std::ops::AddAssign<T> + Copy,
        const BUFFER_SIZE: usize,
        const NI: usize,
    > PipelineNodeOp<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, NI>
    for PointwiseAdder<BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [BufferArray<T, BUFFER_SIZE>; NI],
        output: &mut BufferArray<T, BUFFER_SIZE>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            for input_channel in 0..NI {
                *output.get_mut(idx) += *input[input_channel].get(idx);
            }
        }
        Ok(())
    }
}

pub struct PointwiseSubtractor<const BUFFER_SIZE: usize> {}
impl<const BUFFER_SIZE: usize> PointwiseSubtractor<BUFFER_SIZE> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Sharable + Num + std::ops::SubAssign<T> + Copy, const BUFFER_SIZE: usize, const NI: usize>
    PipelineNodeOp<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, NI>
    for PointwiseSubtractor<BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [BufferArray<T, BUFFER_SIZE>; NI],
        output: &mut BufferArray<T, BUFFER_SIZE>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            for input_channel in 0..NI {
                *output.get_mut(idx) -= *input[input_channel].get(idx);
            }
        }
        Ok(())
    }
}

pub struct PointwiseMultiplier<const BUFFER_SIZE: usize> {}
impl<const BUFFER_SIZE: usize> PointwiseMultiplier<BUFFER_SIZE> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Sharable + Num + std::ops::MulAssign<T> + Copy, const BUFFER_SIZE: usize, const NI: usize>
    PipelineNodeOp<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, NI>
    for PointwiseMultiplier<BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [BufferArray<T, BUFFER_SIZE>; NI],
        output: &mut BufferArray<T, BUFFER_SIZE>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            for input_channel in 0..NI {
                *output.get_mut(idx) *= *input[input_channel].get(idx);
            }
        }
        Ok(())
    }
}

pub struct PointwiseDivider<const BUFFER_SIZE: usize> {}
impl<const BUFFER_SIZE: usize> PointwiseDivider<BUFFER_SIZE> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Sharable + Num + std::ops::DivAssign<T> + Copy, const BUFFER_SIZE: usize, const NI: usize>
    PipelineNodeOp<BufferArray<T, BUFFER_SIZE>, BufferArray<T, BUFFER_SIZE>, NI>
    for PointwiseDivider<BUFFER_SIZE>
{
    fn run_cpu(
        &mut self,
        input: &mut [BufferArray<T, BUFFER_SIZE>; NI],
        output: &mut BufferArray<T, BUFFER_SIZE>,
    ) -> Result<(), ()> {
        for idx in 0..BUFFER_SIZE {
            for input_channel in 0..NI {
                *output.get_mut(idx) /= *input[input_channel].get(idx);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wrapped_buffer<const N: usize>(values: [i32; N]) -> BufferArray<i32, N> {
        BufferArray::new_with_value(values)
    }

    #[test]
    fn pointwise_adder_adds_across_all_inputs() {
        let mut step = PointwiseAdder::<4>::new();
        let mut input = [
            wrapped_buffer([1, 2, 3, 4]),
            wrapped_buffer([10, 20, 30, 40]),
        ];
        let mut output = BufferArray::new();

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read(), &[11, 22, 33, 44]);
    }

    #[test]
    fn pointwise_subtractor_subtracts_across_all_inputs() {
        let mut step = PointwiseSubtractor::<4>::new();
        let mut input = [
            wrapped_buffer([1, 2, 3, 4]),
            wrapped_buffer([10, 20, 30, 40]),
        ];
        let mut output =
            BufferArray::new_with_value([100, 100, 100, 100]);

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read(), &[89, 78, 67, 56]);
    }

    #[test]
    fn pointwise_multiplier_multiplies_across_all_inputs() {
        let mut step = PointwiseMultiplier::<4>::new();
        let mut input = [
            wrapped_buffer([1, 2, 3, 4]),
            wrapped_buffer([10, 20, 30, 40]),
        ];
        let mut output = BufferArray::new_with_value([2, 2, 2, 2]);

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read(), &[20, 80, 180, 320]);
    }

    #[test]
    fn pointwise_divider_divides_across_all_inputs() {
        let mut step = PointwiseDivider::<4>::new();
        let mut input = [wrapped_buffer([2, 5, 3, 4]), wrapped_buffer([5, 2, 4, 8])];
        let mut output =
            BufferArray::new_with_value([100, 100, 96, 128]);

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read(), &[10, 10, 8, 4]);
    }
}
