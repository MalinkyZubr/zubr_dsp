pub fn frequency_from_index(index: usize, sample_frequency: f32, buffer_size: usize) -> f32 {
    index as f32 * sample_frequency / buffer_size as f32
}

pub fn index_from_frequeny(frequency: f32, sample_frequency: f32, buffer_size: usize) -> usize {
    assert!(frequency <= sample_frequency / 2.0);
    (frequency * buffer_size as f32 / sample_frequency) as usize
}
