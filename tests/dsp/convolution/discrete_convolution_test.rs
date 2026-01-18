mod convolution_test {
    use crate::dsp::convolution::discrete_convolution::DiscreteConvolution;
    use crate::pipeline::api::*;
    use super::*;
    extern crate test;


    #[test]
    fn test_convolution() {
        let mut convolver = DiscreteConvolution::new(2, vec![1.0, 1.0, 1.0]);
        let input_vector = vec![0.5, 2.0];
        let result_true = vec![0.5, 2.5];

        let result_exp = convolver.run(ReceiveType::Single(input_vector));

        assert_eq!(result_exp.unwrap().unwrap_noninterleaved(), result_true);
    }
    
    #[bench]
    fn bench_convolution(b: &mut test::Bencher) {
        let mut convolver = DiscreteConvolution::new(1024, vec![1.0; 64]);

        b.iter(|| {
            let input_vector = vec![1.0; 1024];
            convolver.run(ReceiveType::Single(input_vector));
        })
    }
}