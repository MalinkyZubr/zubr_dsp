use num::abs;

pub fn is_within_error_bounds(limit_error: f32, actual: f32, expected: f32) -> bool {
    let percent_error = abs((actual - expected) / expected) * 100.0;

    return percent_error < limit_error;
}


#[cfg(test)]
pub mod filtering_shared {
    use crate::dsp::filtering::iir::{shared::filter::*, tests::filtering_shared::is_within_error_bounds, shared::aberth_stability::*, shared::filter_design_utils::*, shared::tustin_transform::*};
    use num::Complex;

    #[test]
    fn first_order_lowpass_digitizer() {
        let (numerator, denominator) = compute_z_coefficients_o1([1.0,0.0], [1.0, 1.0], 1.0);

        let ideal_numerator = [0.333, 0.333];
        let ideal_denominator = [1.00, -0.333];

        dbg!("{} should be {}\n{} should be {}\n", &numerator, &ideal_numerator, &denominator, &ideal_denominator);

        for index in 0..numerator.len() {
            assert!(is_within_error_bounds(5.0, numerator[index], ideal_numerator[index]));
            assert!(is_within_error_bounds(5.0, denominator[index], ideal_denominator[index]));
        }
    }

    #[test]
    fn second_order_lowpass_digitizer() {
        let (numerator, denominator) = compute_z_coefficients_o2([1.0,0.0,0.0], [1.0, ((2.0 as f32).sqrt()), 1.0], 1.0);

        let ideal_numerator = [0.1277, 0.2555, 0.1277];
        let ideal_denominator = [1.0, -0.7664, 0.2774];

        dbg!("{} should be {}\n{} should be {}\n", &numerator, &ideal_numerator, &denominator, &ideal_denominator);

        for index in 0..numerator.len() {
            assert!(is_within_error_bounds(5.0, numerator[index], ideal_numerator[index]));
            assert!(is_within_error_bounds(5.0, denominator[index], ideal_denominator[index]));
        }
    }

    #[test]
    fn third_order_lowpass_digitizer() {
        let (numerator, denominator) = compute_z_coefficients_o3([1.0,0.0,0.0, 0.0], [1.0, 2.0, 2.0, 1.0], 1.0);

        let ideal_numerator = [0.04762, 0.1429, 0.1429, 0.04762];
        let ideal_denominator = [1.0, -1.19, 0.7143, -0.1429];

        dbg!("{} should be {}\n{} should be {}\n", &numerator, &ideal_numerator, &denominator, &ideal_denominator);

        for index in 0..numerator.len() {
            assert!(is_within_error_bounds(5.0, numerator[index], ideal_numerator[index]));
            assert!(is_within_error_bounds(5.0, denominator[index], ideal_denominator[index]));
        }
    }

    #[test]
    fn test_cauchy_root_bounding() {
        let polynomial = vec![-3.0, -6.0, -1.0, 4.0, 2.0];
        assert!(identify_root_bounds(&polynomial) == (-4.0, 4.0));

        let polynomial = vec![5.0, -7.0, 1.0, 3.0, -4.0, 2.0];
        assert!(identify_root_bounds(&polynomial) == (-4.5, 4.5));
    }

    #[test]
    fn test_coefficient_derivative() {
        let polynomial = vec![-3.0, -6.0, -1.0, 4.0, 2.0];
        assert!(generate_derivative_coefficients(&polynomial) == vec![-6.0, -2.0, 12.0, 8.0]);

        let polynomial = vec![5.0, -7.0, 1.0, 3.0, -4.0, 2.0];
        assert!(generate_derivative_coefficients(&polynomial) == vec![-7.0, 2.0, 9.0, -16.0, 10.0]);
    }

    #[test]
    fn test_initial_guess_generation() {
        for _test_num in 0..1000 {
            let mut initial_estimates: Vec<Complex<f32>> = Vec::new();
            generate_initial_estimations(&(-5.0, 5.0), &mut initial_estimates, 10);

            for value in initial_estimates {
                //dbg!("{}", value);
                assert!(value.norm() <= 5.0);
            }
        }
    }

    #[test]
    fn test_polynomial_computation() {
        let polynomial = vec![-3.0, -6.0, -1.0, 4.0, 2.0];
        dbg!("{}", compute_polynomial_image(&Complex::new(1.0, 0.0), &polynomial).norm());
        assert!(
            is_within_error_bounds(
                5.0,
                compute_polynomial_image(&Complex::new(1.0, 0.0), &polynomial).re, 
                -4.0, 
            )
        );
    }

    fn check_if_match_any(collection: &Vec<f32>, observed: &f32) -> bool {
        for value in collection.iter() {
            if is_within_error_bounds(5.0, *observed, *value) {
                return true;
            }
        }

        return false;
    }

    #[test]
    fn test_aberth() {
        let polynomial = vec![1.0, 2.0, -3.0];
        let roots: Vec<Complex<f32>> = compute_polynomial_roots(&polynomial, 0.001);
        let ideal_roots = vec![-0.33333, 1.0];

        for root in roots.iter() {
            assert!(check_if_match_any(&ideal_roots, &root.re))
        }
    }
}