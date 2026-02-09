pub mod FIRImpulseResponseTests {
    use num::Complex;
    use crate::dsp::fft::fftshift::*;
    use crate::dsp::filtering::fir::ideal_response::fir_utils::FIRTransferFunction;
    use crate::dsp::filtering::fir::ideal_response::rectangles::{SelectPassRectangular, SelectPassType, SidePassRectangular, SidePassType};

    fn convert_to_complex(input: Vec<f32>) -> Vec<Complex<f32>> {
        let new_vector = input.iter()
            .map(|value| Complex::new(*value, 0.0))
            .collect();

        return new_vector;
    }

    #[test]
    pub fn test_filter_frequency_responses() {
        let mut test_frequency_spectrum = generate_frequency_axis(10.0, 10);
        fft_shift(&mut test_frequency_spectrum);

        let low_pass = SidePassRectangular::new(1.0, 10.0, SidePassType::LowPass);
        let low_pass_filter = low_pass.transfer_function(test_frequency_spectrum.clone());

        assert!(low_pass_filter == convert_to_complex(
            vec![0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]
        ));

        let high_pass = SidePassRectangular::new(2.0, 10.0, SidePassType::HighPass);
        let high_pass_filter = high_pass.transfer_function(test_frequency_spectrum.clone());
        
        dbg!(&high_pass_filter);
        assert!(high_pass_filter == convert_to_complex(
            vec![1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        ));
        
        let band_pass = SelectPassRectangular::new(3.0, 3.0, 10.0, SelectPassType::BandPass);
        let band_pass_filter = band_pass.transfer_function(test_frequency_spectrum.clone());
        
        dbg!(&band_pass_filter);
        assert!(band_pass_filter == convert_to_complex(
            vec![0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
        ));

        let band_stop = SelectPassRectangular::new(3.0, 3.0, 10.0, SelectPassType::BandStop);
        let band_stop_filter = band_stop.transfer_function(test_frequency_spectrum.clone());
        
        dbg!(&band_stop_filter);
        assert!(band_stop_filter == convert_to_complex(
            vec![1.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0]
        ));
    }
}