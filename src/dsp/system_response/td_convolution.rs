use crate::pipeline::communication_layer::data_management::*;
use crate::pipeline::construction_layer::node_types::pipeline_step::*;
use crate::pipeline::construction_layer::pipeline_traits::*;
use num::Num;
use std::iter::Sum;
use std::mem;

const fn max(a: usize, b: usize) -> usize {
    if a > b {
        a
    } else {
        b
    }
}

pub struct DiscreteConvolution<T: Sharable + Num + Sum, const IRS: usize, const IS: usize>
where
{
    impulse_response: BufferArray<T, IRS>,
    internal_buffer: [T; IS],
    window_buffer: [T; IRS],
}

impl<T: Sharable + Num + Copy + Sum, const IRS: usize, const IS: usize>
    DiscreteConvolution<T, IRS, IS>
{
    pub fn new(mut impulse_response: BufferArray<T, IRS>) -> Self {
        impulse_response.reverse();
        Self {
            impulse_response,
            internal_buffer: [T::zero(); IS],
            window_buffer: [T::zero(); IRS],
        }
    }

    fn convolve_input(&mut self, input: &mut BufferArray<T, IS>) {
        let end = self.window_buffer.len() - 1;

        for output_index in 0..IS {
            self.window_buffer.rotate_left(1); // inefficient. Replace with indexing later
            self.window_buffer[end] = *input.get(output_index);

            self.internal_buffer[output_index] = self.window_buffer
                .iter()
                .zip(self.impulse_response.read())
                .map(|(input_sample, window_sample)| *input_sample * *window_sample)
                .sum();
        }
    }
}

impl<T: Sharable + Num + Sum, const IRS: usize, const IS: usize>
    PipelineStep<BufferArray<T, IS>, BufferArray<T, IS>, 1>
    for DiscreteConvolution<T, IRS, IS>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, IS>>; 1],
        output: &mut DataWrapper<BufferArray<T, IS>>,
    ) -> Result<(), ()>
    {
        self.convolve_input(input[0].read());
        mem::swap(output.read().read_mut(), &mut self.internal_buffer);

        Ok(())
    }
}

#[cfg(test)]
mod td_convolution_tests {
    use crate::pipeline::communication_layer::data_management::BufferArray;

    #[test]
    fn test_td_convolution_ir_eq_in() {
        let mut input = BufferArray::new_with_value([1, 2, 3, 4, 5]);
        let impulse_response = BufferArray::new_with_value([1, 2, 3, 4, 5]);

        let mut td_convolution = super::DiscreteConvolution::new(impulse_response);
        td_convolution.convolve_input(&mut input);

        assert_eq!(td_convolution.impulse_response.read(), &[5, 4, 3, 2, 1]);
        assert_eq!(td_convolution.internal_buffer, [1, 4, 10, 20, 35]);
        
        let mut input = BufferArray::new_with_value([6, 7, 8, 9, 0]);
        td_convolution.convolve_input(&mut input);
        
        assert_eq!(td_convolution.internal_buffer, [50, 65, 80, 95, 100]);
    }

    #[test]
    fn test_td_convolution_ir_gt_in() {
        let mut input = BufferArray::new_with_value([1,3,5]);
        let impulse_response = BufferArray::new_with_value([29, 1, 58, 30, 99, 8, 50, 90, 220, 250, 29, 1, 58, 30, 99, 8, 50, 90, 220, 250]);

        let mut td_convolution = super::DiscreteConvolution::new(impulse_response);
        td_convolution.convolve_input(&mut input);
        
        assert_eq!(td_convolution.internal_buffer, [29, 88, 206]);
        
        let mut input = BufferArray::new_with_value([2, 2, 3]);
        td_convolution.convolve_input(&mut input);
        
        assert_eq!(td_convolution.internal_buffer, [267 ,539, 660]);
        
        let mut input = BufferArray::new_with_value([9, 9, 5]);
        td_convolution.convolve_input(&mut input);
        
        assert_eq!(td_convolution.internal_buffer, [1009, 982, 1720]);
        
    }

    #[test]
    fn test_td_convolution_ir_lt_in() {
        let mut input = BufferArray::new_with_value([1, 2, 3, 4, 5]);
        let impulse_response = BufferArray::new_with_value([1, 2]);

        let mut td_convolution = super::DiscreteConvolution::new(impulse_response);
        td_convolution.convolve_input(&mut input);

        assert_eq!(td_convolution.internal_buffer, [1, 4, 7, 10, 13]);

        let mut input = BufferArray::new_with_value([6, 7, 8, 9, 0]);
        td_convolution.convolve_input(&mut input);
        
        assert_eq!(td_convolution.internal_buffer, [16, 19, 22, 25, 18])
    }
}
