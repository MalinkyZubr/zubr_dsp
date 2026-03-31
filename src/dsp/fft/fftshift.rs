use std::mem;

pub fn fft_shift<T>(buffer: &mut Vec<T>) {
    let length = buffer.len();
    let (positive_frequencies, negative_frequencies) = buffer.split_at_mut(length / 2);

    for (positive_frequency, negative_frequency) in
        positive_frequencies.iter_mut().zip(negative_frequencies)
    {
        mem::swap(positive_frequency, negative_frequency);
    }
}

pub fn generate_frequency_axis(sampling_rate: f32, buffer_size: usize) -> Vec<f32> {
    let mut frequency_axis = Vec::with_capacity(buffer_size);
    let mut negative_buffer = Vec::with_capacity(buffer_size / 2);

    for bin_num in 0..buffer_size / 2 {
        frequency_axis.push(bin_num as f32 * sampling_rate / buffer_size as f32);
    }
    for bin_num in 1..(buffer_size / 2) + 1 {
        negative_buffer.push(-(bin_num as f32) * sampling_rate / buffer_size as f32);
    }

    negative_buffer.reverse();
    frequency_axis.append(&mut negative_buffer);

    return frequency_axis;
}
