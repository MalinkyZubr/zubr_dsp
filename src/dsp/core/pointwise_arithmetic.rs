use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;
use std::iter::Sum;

pub struct PointwiseAdder<const BufferSize: usize> {}
impl<const BufferSize: usize> PointwiseAdder<BufferSize> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<
        T: Sharable + Num + Sum + std::ops::AddAssign<T>,
        const BufferSize: usize,
        const NI: usize,
    > PipelineStep<BufferArray<T, BufferSize>, BufferArray<T, BufferSize>, NI>
    for PointwiseAdder<BufferSize>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BufferSize>>; NI],
        output: &mut DataWrapper<BufferArray<T, BufferSize>>,
    ) -> Result<(), ()> {
        for idx in 0..BufferSize {
            for input_channel in 0..NI {
                *output.read().get_mut(idx) += *input[input_channel].read().get(idx);
            }
        }
        Ok(())
    }
}

pub struct PointwiseSubtractor<const BufferSize: usize> {}
impl<const BufferSize: usize> PointwiseSubtractor<BufferSize> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Sharable + Num + std::ops::SubAssign<T>, const BufferSize: usize, const NI: usize>
    PipelineStep<BufferArray<T, BufferSize>, BufferArray<T, BufferSize>, NI>
    for PointwiseSubtractor<BufferSize>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BufferSize>>; NI],
        output: &mut DataWrapper<BufferArray<T, BufferSize>>,
    ) -> Result<(), ()> {
        for idx in 0..BufferSize {
            for input_channel in 0..NI {
                *output.read().get_mut(idx) -= *input[input_channel].read().get(idx);
            }
        }
        Ok(())
    }
}

pub struct PointwiseMultiplier<const BufferSize: usize> {}
impl<const BufferSize: usize> PointwiseMultiplier<BufferSize> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Sharable + Num + std::ops::MulAssign<T>, const BufferSize: usize, const NI: usize>
    PipelineStep<BufferArray<T, BufferSize>, BufferArray<T, BufferSize>, NI>
    for PointwiseMultiplier<BufferSize>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BufferSize>>; NI],
        output: &mut DataWrapper<BufferArray<T, BufferSize>>,
    ) -> Result<(), ()> {
        for idx in 0..BufferSize {
            for input_channel in 0..NI {
                *output.read().get_mut(idx) *= *input[input_channel].read().get(idx);
            }
        }
        Ok(())
    }
}

pub struct PointwiseDivider<const BufferSize: usize> {}
impl<const BufferSize: usize> PointwiseDivider<BufferSize> {
    pub fn new() -> Self {
        Self {}
    }
}
impl<T: Sharable + Num + std::ops::DivAssign<T>, const BufferSize: usize, const NI: usize>
    PipelineStep<BufferArray<T, BufferSize>, BufferArray<T, BufferSize>, NI>
    for PointwiseDivider<BufferSize>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BufferSize>>; NI],
        output: &mut DataWrapper<BufferArray<T, BufferSize>>,
    ) -> Result<(), ()> {
        for idx in 0..BufferSize {
            for input_channel in 0..NI {
                *output.read().get_mut(idx) /= *input[input_channel].read().get(idx);
            }
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
    fn pointwise_adder_adds_across_all_inputs() {
        let mut step = PointwiseAdder::<4>::new();
        let mut input = [
            wrapped_buffer([1, 2, 3, 4]),
            wrapped_buffer([10, 20, 30, 40]),
        ];
        let mut output = DataWrapper::new_with_value(BufferArray::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[11, 22, 33, 44]);
    }

    #[test]
    fn pointwise_subtractor_subtracts_across_all_inputs() {
        let mut step = PointwiseSubtractor::<4>::new();
        let mut input = [
            wrapped_buffer([1, 2, 3, 4]),
            wrapped_buffer([10, 20, 30, 40]),
        ];
        let mut output =
            DataWrapper::new_with_value(BufferArray::new_with_value([100, 100, 100, 100]));

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[89, 78, 67, 56]);
    }

    #[test]
    fn pointwise_multiplier_multiplies_across_all_inputs() {
        let mut step = PointwiseMultiplier::<4>::new();
        let mut input = [
            wrapped_buffer([1, 2, 3, 4]),
            wrapped_buffer([10, 20, 30, 40]),
        ];
        let mut output = DataWrapper::new_with_value(BufferArray::new_with_value([2, 2, 2, 2]));

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[20, 80, 180, 320]);
    }

    #[test]
    fn pointwise_divider_divides_across_all_inputs() {
        let mut step = PointwiseDivider::<4>::new();
        let mut input = [wrapped_buffer([2, 5, 3, 4]), wrapped_buffer([5, 2, 4, 8])];
        let mut output =
            DataWrapper::new_with_value(BufferArray::new_with_value([100, 100, 96, 128]));

        step.run_cpu(&mut input, &mut output).unwrap();

        assert_eq!(output.read().read(), &[10, 10, 8, 4]);
    }
}
