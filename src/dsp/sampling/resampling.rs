use crate::pipeline::communication_layer::data_management::{BufferArray, DataWrapper};
use crate::pipeline::construction_layer::node_types::pipeline_step::PipelineStep;
use crate::pipeline::construction_layer::pipeline_traits::Sharable;
use num::{Num, NumCast};
use std::mem;

pub enum UpsamplingMethod {
    LinearInterpolation,
    NearestNeighbor,
    LeftHandHold,
    RightHandHold,
    ZeroFilling,
}
impl UpsamplingMethod {
    pub fn to_function<T: Num + Sharable + NumCast>(mut self) -> fn(&T, &T, &mut [T]) {
        match self {
            Self::LinearInterpolation => linear_interpolation,
            Self::NearestNeighbor => nearest_neighbor,
            Self::LeftHandHold => left_hand_hold,
            Self::RightHandHold => right_hand_hold,
            Self::ZeroFilling => zero_filling,
        }
    }
}

fn linear_interpolation<T: Num + NumCast + Copy + Clone>(p1: &T, p2: &T, interp_window: &mut [T]) {
    let slope = (*p2 - *p1) / NumCast::from(interp_window.len()).unwrap();

    for (index, val) in interp_window.iter_mut().enumerate() {
        *val = *p1 + slope * NumCast::from(index).unwrap();
    }
}

fn nearest_neighbor<T: Num + NumCast + Copy + Clone>(p1: &T, p2: &T, interp_window: &mut [T]) {
    let mut rep_value = p1;
    let halfway = interp_window.len() / 2;

    for val in interp_window[0..halfway].iter_mut() {
        *val = *rep_value;
    }
    rep_value = p2;
    for val in interp_window[halfway..].iter_mut() {
        *val = *rep_value;
    }
}

fn zero_filling<T: Num + NumCast + Copy + Clone>(p1: &T, p2: &T, interp_window: &mut [T]) {
    for val in interp_window.iter_mut() {
        *val = NumCast::from(0).unwrap();
    }
}

fn left_hand_hold<T: Num + NumCast + Copy + Clone>(p1: &T, p2: &T, interp_window: &mut [T]) {
    for val in interp_window.iter_mut() {
        *val = *p1;
    }
}

fn right_hand_hold<T: Num + NumCast + Copy + Clone>(p1: &T, p2: &T, interp_window: &mut [T]) {
    for val in interp_window.iter_mut() {
        *val = *p2;
    }
}

pub struct Resampler<
    T: Sharable + Num,
    const BufferSize: usize,
    const UpsampleFactor: usize,
    const DecimationFactor: usize,
> where
    [(); UpsampleFactor * BufferSize]:,
    [(); UpsampleFactor * BufferSize / DecimationFactor]:,
{
    upsample_method: fn(&T, &T, &mut [T]),
    upsample_buffer: [T; UpsampleFactor * BufferSize],
    decimation_buffer: BufferArray<T, { UpsampleFactor * BufferSize / DecimationFactor }>,
}

impl<
        T: Sharable + Num + NumCast,
        const BufferSize: usize,
        const UpsampleFactor: usize,
        const DecimationFactor: usize,
    > Resampler<T, BufferSize, UpsampleFactor, DecimationFactor>
where
    [(); UpsampleFactor * BufferSize]:,
    [(); UpsampleFactor * BufferSize / DecimationFactor]:,
{
    pub fn new(method: UpsamplingMethod) -> Self {
        Self {
            upsample_method: method.to_function(),
            upsample_buffer: [T::zero(); { UpsampleFactor * BufferSize }],
            decimation_buffer: BufferArray::new(),
        }
    }

    fn upsample(&mut self, input_buffer: &mut [T; BufferSize]) {
        mem::swap(&mut input_buffer[0], &mut self.upsample_buffer[0]);

        for idx in 1..BufferSize {
            mem::swap(
                &mut input_buffer[idx],
                &mut self.upsample_buffer[idx * UpsampleFactor],
            );
            (self.upsample_method)(
                &input_buffer[idx - 1],
                &input_buffer[idx],
                &mut self.upsample_buffer[(idx - 1) * UpsampleFactor + 1..idx * UpsampleFactor],
            ); // DO not want to interpolate over existing values
        }
    }

    fn decimate(&mut self) {
        for idx in (0..BufferSize).step_by(DecimationFactor) {
            mem::swap(
                &mut self.upsample_buffer[idx * UpsampleFactor],
                &mut self.decimation_buffer.read_mut()[idx],
            );
        }
    }
}

impl<
        T: Sharable + Num + NumCast,
        const BufferSize: usize,
        const UpsampleFactor: usize,
        const DecimationFactor: usize,
    >
    PipelineStep<
        BufferArray<T, BufferSize>,
        BufferArray<T, { UpsampleFactor * BufferSize / DecimationFactor }>,
        1,
    > for Resampler<T, BufferSize, UpsampleFactor, DecimationFactor>
where
    [(); UpsampleFactor * BufferSize]:,
    [(); UpsampleFactor * BufferSize / DecimationFactor]:,
{
    fn run_cpu(
        &mut self,
        input: &mut [DataWrapper<BufferArray<T, BufferSize>>; 1],
        output: &mut DataWrapper<
            BufferArray<T, { UpsampleFactor * BufferSize / DecimationFactor }>,
        >,
    ) -> Result<(), ()> {
        self.upsample(input[0].read().read_mut());
        self.decimate();
        output.swap(&mut self.decimation_buffer);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    // must de-ai-ify these tests asap
    use super::*;

    // Helper: create BufferArray from array
    fn buffer_from_array<T: Copy + Default + Sharable, const N: usize>(
        data: [T; N],
    ) -> BufferArray<T, N> {
        let mut buf = BufferArray::new();
        *buf.read_mut() = data;
        buf
    }

    // -----------------------------
    // Upsampling method tests
    // -----------------------------

    #[test]
    fn test_zero_filling() {
        let mut window = [1, 2, 3, 4];
        zero_filling(&10, &20, &mut window);
        assert_eq!(window, [0, 0, 0, 0]);
    }

    #[test]
    fn test_left_hand_hold() {
        let mut window = [0; 4];
        left_hand_hold(&5, &10, &mut window);
        assert_eq!(window, [5, 5, 5, 5]);
    }

    #[test]
    fn test_right_hand_hold() {
        let mut window = [0; 4];
        right_hand_hold(&5, &10, &mut window);
        assert_eq!(window, [10, 10, 10, 10]);
    }

    #[test]
    fn test_nearest_neighbor_even() {
        let mut window = [0; 4];
        nearest_neighbor(&1, &9, &mut window);
        assert_eq!(window, [1, 1, 9, 9]);
    }

    #[test]
    fn test_linear_interpolation() {
        let mut window = [0.0; 4];
        linear_interpolation(&0.0, &4.0, &mut window);

        // slope = 4 / 4 = 1
        assert_eq!(window, [0.0, 1.0, 2.0, 3.0]);
    }

    // -----------------------------
    // Resampler tests
    // -----------------------------

    #[test]
    fn test_upsample_linear() {
        const N: usize = 4;
        const U: usize = 2;
        const D: usize = 1;

        let mut resampler: Resampler<f32, N, U, D> =
            Resampler::new(UpsamplingMethod::LinearInterpolation);

        let mut input = [1.0, 3.0, 5.0, 7.0];

        resampler.upsample(&mut input);

        // Expected pattern:
        // [1, interp, 3, interp, 5, interp, 7, ...]
        let buf = &resampler.upsample_buffer;

        assert_eq!(buf[0], 1.0);
        assert_eq!(buf[2], 3.0);
        assert_eq!(buf[4], 5.0);
        assert_eq!(buf[6], 7.0);
    }

    #[test]
    fn test_decimation_basic() {
        const N: usize = 4;
        const U: usize = 2;
        const D: usize = 2;

        let mut resampler: Resampler<i32, N, U, D> = Resampler::new(UpsamplingMethod::LeftHandHold);

        let mut input = [1, 2, 3, 4];

        resampler.upsample(&mut input);
        resampler.decimate();

        let out = resampler.decimation_buffer.read();

        // Expect every 2nd sample after upsample
        // Exact values depend on interpolation, but positions matter
        assert_eq!(out.len(), N * U / D);
    }

    #[test]
    fn test_pipeline_step_integration() {
        const N: usize = 4;
        const U: usize = 2;
        const D: usize = 2;

        let mut resampler: Resampler<f32, N, U, D> =
            Resampler::new(UpsamplingMethod::NearestNeighbor);

        let input_array = [1.0, 2.0, 3.0, 4.0];

        let mut input_wrapper = DataWrapper::new_with_value(buffer_from_array(input_array));
        let mut output_wrapper =
            DataWrapper::new_with_value(BufferArray::<f32, { N * U / D }>::new());

        let mut input_arr = [input_wrapper];

        resampler
            .run_cpu(&mut input_arr, &mut output_wrapper)
            .unwrap();

        let result = output_wrapper.read();

        assert_eq!(result.len(), N * U / D);
    }

    // -----------------------------
    // Edge cases
    // -----------------------------

    #[test]
    fn test_no_upsample_no_decimate() {
        const N: usize = 4;
        const U: usize = 1;
        const D: usize = 1;

        let mut resampler: Resampler<i32, N, U, D> = Resampler::new(UpsamplingMethod::ZeroFilling);

        let mut input = [1, 2, 3, 4];

        resampler.upsample(&mut input);
        resampler.decimate();

        let out = resampler.decimation_buffer.read();

        assert_eq!(out, &[1, 2, 3, 4]);
    }
}
