use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use num::Complex;
use num::Num;
use std::mem;

pub struct RealToComplex<T: Sharable + Num, const BS: usize> {
    internal_buffer: BufferArray<Complex<T>, BS>,
}
impl<T: Sharable + Num, const BS: usize> RealToComplex<T, BS> {
    pub fn new() -> Self {
        Self {
            internal_buffer: BufferArray::new(),
        }
    }
}
impl<T: Sharable + Num, const BS: usize>
    PipelineStep<BufferArray<T, BS>, BufferArray<Complex<T>, BS>, 1> for RealToComplex<T, BS>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BS>>; 1],
        output: &mut DataWrapper<BufferArray<Complex<T>, BS>>,
    ) -> Result<(), ()> {
        let direct_ref = input[0].read().read_mut();
        for idx in 0..BS {
            mem::swap(
                &mut self.internal_buffer.read_mut()[idx].re,
                &mut direct_ref[idx],
            );
        }

        output.swap(&mut self.internal_buffer);

        Ok(())
    }
}

pub struct ImagToComplex<T: Sharable + Num, const BS: usize> {
    internal_buffer: BufferArray<Complex<T>, BS>,
}
impl<T: Sharable + Num, const BS: usize> ImagToComplex<T, BS> {
    pub fn new() -> Self {
        Self {
            internal_buffer: BufferArray::new(),
        }
    }
}
impl<T: Sharable + Num, const BS: usize>
    PipelineStep<BufferArray<T, BS>, BufferArray<Complex<T>, BS>, 1> for ImagToComplex<T, BS>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BS>>; 1],
        output: &mut DataWrapper<BufferArray<Complex<T>, BS>>,
    ) -> Result<(), ()> {
        let direct_ref = input[0].read().read_mut();
        for idx in 0..BS {
            mem::swap(
                &mut self.internal_buffer.read_mut()[idx].im,
                &mut direct_ref[idx],
            );
        }

        output.swap(&mut self.internal_buffer);

        Ok(())
    }
}

pub struct ComposeComplex<T: Sharable + Num, const BS: usize> {
    internal_buffer: BufferArray<Complex<T>, BS>,
}
impl<T: Sharable + Num, const BS: usize> ComposeComplex<T, BS> {
    pub fn new() -> Self {
        Self {
            internal_buffer: BufferArray::new(),
        }
    }
}
impl<T: Sharable + Num, const BS: usize>
    PipelineStep<BufferArray<T, BS>, BufferArray<Complex<T>, BS>, 2> for ComposeComplex<T, BS>
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BS>>; 2],
        output: &mut DataWrapper<BufferArray<Complex<T>, BS>>,
    ) -> Result<(), ()> {
        for idx in 0..BS {
            mem::swap(
                &mut self.internal_buffer.read_mut()[idx].re,
                &mut input[0].read().read_mut()[idx],
            );
            mem::swap(
                &mut self.internal_buffer.read_mut()[idx].im,
                &mut input[1].read().read_mut()[idx],
            );
        }

        output.swap(&mut self.internal_buffer);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num::Complex;

    // Helper
    fn buffer_from_array<T: Copy + Default + Sharable, const N: usize>(
        data: [T; N],
    ) -> BufferArray<T, N> {
        let mut buf = BufferArray::new();
        *buf.read_mut() = data;
        buf
    }

    // -----------------------------
    // RealToComplex
    // -----------------------------

    #[test]
    fn test_real_to_complex_basic() {
        const N: usize = 4;

        let mut step: RealToComplex<i32, N> = RealToComplex::new();

        let input_data = [1, 2, 3, 4];
        let mut input = [DataWrapper::new_with_value(buffer_from_array(input_data))];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        let out = output.read();

        for i in 0..N {
            assert_eq!(out.read()[i].re, input_data[i]);
            assert_eq!(out.read()[i].im, 0);
        }
    }

    #[test]
    fn test_real_to_complex_input_mutated() {
        const N: usize = 4;

        let mut step: RealToComplex<i32, N> = RealToComplex::new();

        let input_data = [5, 6, 7, 8];
        let mut input = [DataWrapper::new_with_value(buffer_from_array(input_data))];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        // After swap, input should now contain zeros (initial internal buffer values)
        let mutated = input[0].read();

        for i in 0..N {
            assert_eq!(mutated.read()[i], 0);
        }
    }

    // -----------------------------
    // ImagToComplex
    // -----------------------------

    #[test]
    fn test_imag_to_complex_basic() {
        const N: usize = 4;

        let mut step: ImagToComplex<i32, N> = ImagToComplex::new();

        let input_data = [1, 2, 3, 4];
        let mut input = [DataWrapper::new_with_value(buffer_from_array(input_data))];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        let out = output.read();

        for i in 0..N {
            assert_eq!(out.read()[i].re, 0);
            assert_eq!(out.read()[i].im, input_data[i]);
        }
    }

    #[test]
    fn test_imag_to_complex_input_mutated() {
        const N: usize = 4;

        let mut step: ImagToComplex<i32, N> = ImagToComplex::new();

        let input_data = [9, 8, 7, 6];
        let mut input = [DataWrapper::new_with_value(buffer_from_array(input_data))];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        let mutated = input[0].read();

        for i in 0..N {
            assert_eq!(mutated.read()[i], 0);
        }
    }

    // -----------------------------
    // ComposeComplex
    // -----------------------------

    #[test]
    fn test_compose_complex_basic() {
        const N: usize = 4;

        let mut step: ComposeComplex<i32, N> = ComposeComplex::new();

        let real = [1, 2, 3, 4];
        let imag = [5, 6, 7, 8];

        let mut input = [
            DataWrapper::new_with_value(buffer_from_array(real)),
            DataWrapper::new_with_value(buffer_from_array(imag)),
        ];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        let out = output.read();

        for i in 0..N {
            assert_eq!(out.read()[i], Complex::new(real[i], imag[i]));
        }
    }

    #[test]
    fn test_compose_complex_inputs_mutated() {
        const N: usize = 4;

        let mut step: ComposeComplex<i32, N> = ComposeComplex::new();

        let real = [10, 20, 30, 40];
        let imag = [1, 2, 3, 4];

        let mut input = [
            DataWrapper::new_with_value(buffer_from_array(real)),
            DataWrapper::new_with_value(buffer_from_array(imag)),
        ];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        for i in 0..N {
            assert_eq!(input[0].read().read()[i], 0);
            assert_eq!(input[1].read().read()[i], 0);
        }
    }

    // -----------------------------
    // Edge case
    // -----------------------------

    #[test]
    fn test_compose_complex_zero_input() {
        const N: usize = 4;

        let mut step: ComposeComplex<i32, N> = ComposeComplex::new();

        let real = [0; N];
        let imag = [0; N];

        let mut input = [
            DataWrapper::new_with_value(buffer_from_array(real)),
            DataWrapper::new_with_value(buffer_from_array(imag)),
        ];

        let mut output = DataWrapper::new_with_value(BufferArray::<Complex<i32>, N>::new());

        step.run_cpu(&mut input, &mut output).unwrap();

        let out = output.read();

        for i in 0..N {
            assert_eq!(out.read()[i], Complex::new(0, 0));
        }
    }
}
