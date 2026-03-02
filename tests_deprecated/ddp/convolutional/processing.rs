#[cfg(test)]
pub mod convolutional_tests {
    use crate::ddp::codings::convolutional::trellis::{
        params::ConvolutionalParams, reconstruct::ConvolutionalReassembler, trellis::{ConvolutionalEncoderLookup, ConvolutionalLookupGenerator, TrellisStateChangeEncode}, viterbi::ViterbiOpCore
    };
    use std::collections::HashMap;
    use rand::Rng;
    extern crate test;
    
    
    #[test]
    fn encoding_trellis_test() {
        let test_params1 = ConvolutionalParams::new(
            2, 
            1, 
            vec![1, 3]);

        let test_trellis: ConvolutionalEncoderLookup = ConvolutionalLookupGenerator::generate_encoding_lookup(&test_params1.unwrap());
    
        let mut reference_lookup: HashMap<u8, HashMap<u8, TrellisStateChangeEncode>> = HashMap::new();
        reference_lookup.insert(0, [
            (0 as u8, TrellisStateChangeEncode{new_state: 0, output: 0}),
            (1 as u8, TrellisStateChangeEncode{new_state: 1, output: 3})
        ].into_iter().collect());

        reference_lookup.insert(1, [
            (0 as u8, TrellisStateChangeEncode{new_state: 2, output: 2}),
            (1 as u8, TrellisStateChangeEncode{new_state: 3, output: 1})
        ].into_iter().collect());

        reference_lookup.insert(2, [
            (0 as u8, TrellisStateChangeEncode{new_state: 0, output: 0}),
            (1 as u8, TrellisStateChangeEncode{new_state: 1, output: 3})
        ].into_iter().collect());

        reference_lookup.insert(3, [
            (0 as u8, TrellisStateChangeEncode{new_state: 2, output: 2}),
            (1 as u8, TrellisStateChangeEncode{new_state: 3, output: 1})
        ].into_iter().collect());

        dbg!("{}", &test_trellis.encoding_lookup);
        assert!(reference_lookup == test_trellis.encoding_lookup)
    }

    #[test]
    fn matrix_gen_test() {
        let test_params1 = ConvolutionalParams::new(
            2, 
            1, 
            vec![1, 3]);

        let test_trellis: ConvolutionalEncoderLookup = ConvolutionalLookupGenerator::generate_encoding_lookup(&test_params1.unwrap());

        let transition_matrix = test_trellis.to_transition_matrix();

        dbg!("{}", &transition_matrix);
        assert!(transition_matrix == vec![
            vec![1, 1, 0, 0], 
            vec![0, 0, 1, 1], 
            vec![1, 1, 0, 0], 
            vec![0, 0, 1, 1]]);

        let emission_matrix = test_trellis.to_emission_matrix();

        dbg!("{}", &emission_matrix);
        assert!(emission_matrix == vec![
            vec![0, 3, 256, 256], 
            vec![256, 256, 2, 1], 
            vec![0, 3, 256, 256], 
            vec![256, 256, 2, 1]]);
    }

    #[test]
    fn viterbi_test() {
        let test_params1 = ConvolutionalParams::new(
            2, 
            1, 
            vec![1, 3]);

        let test_trellis: ConvolutionalEncoderLookup = ConvolutionalLookupGenerator::generate_encoding_lookup(&test_params1.unwrap());
        let mut test_reassembler: ConvolutionalReassembler = ConvolutionalReassembler::new(1); 
        let mut viterbi: ViterbiOpCore = ViterbiOpCore::new(5, &test_trellis);
        let test_output: Vec<u8> = vec![0b00, 0b11, 0b01, 0b10, 0b00];
        //let test_output: Vec<u8> = vec![0b11, 0b11, 0b00, 0b11, 0b00];
        let valid_state_sequence: Vec<u8> = vec![0b00, 0b01, 0b11, 0b10, 0b00];
        let valid_input_sequence: Vec<u8> = vec![0, 1, 1, 0, 0];

        let result = viterbi.viterbi(&test_output);

        let mut input_sequence = vec![0; result.1.len()];
        test_reassembler.compute_input_vector(&result.1, &mut input_sequence);

        dbg!("{}, {}", &result.0, &result.1);
        assert!(result.1 == valid_state_sequence);

        dbg!("{}", &input_sequence);
        assert!(input_sequence == valid_input_sequence);

        let test_output: Vec<u8> = vec![0b11, 0b10, 0b00, 0b00, 0b11];
        let valid_state_sequence: Vec<u8> = vec![0b01, 0b10, 0b00, 0b00, 0b01];
        let valid_input_sequence: Vec<u8> = vec![1, 0, 0, 0, 1];

        let result = viterbi.viterbi(&test_output);

        let mut input_sequence = vec![0; result.1.len()];
        test_reassembler.compute_input_vector(&result.1, &mut input_sequence);

        dbg!("{}, {}", &result.0, &result.1);
        assert!(result.1 == valid_state_sequence);

        dbg!("{}", &input_sequence);
        assert!(input_sequence == valid_input_sequence);
    }
    
    #[bench]
    fn viterbi_bench(b: &mut test::Bencher) {
        let mut rng = rand::rng();
        let mut input = [0; 2048];
        for value in input.iter_mut() {
            *value = rng.random_range(0..4)    
        }
        
        let test_params1 = ConvolutionalParams::new(
            2, 
            1, 
            vec![1, 3]);

        let test_trellis: ConvolutionalEncoderLookup = ConvolutionalLookupGenerator::generate_encoding_lookup(&test_params1.unwrap());
        let mut viterbi: ViterbiOpCore = ViterbiOpCore::new(2048, &test_trellis);
        
        dbg!("{}", &input);
        b.iter(|| {
            let decoded = viterbi.viterbi(&input);
        })
    }
}

