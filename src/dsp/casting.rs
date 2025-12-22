use num::Complex;
use crate::pipeline::interfaces::PipelineStep;
use crate::pipeline::interfaces::ODFormat;

pub struct ComplexCaster {}
impl PipelineStep<Vec<f32>, Vec<Complex<f32>>> for ComplexCaster {
    fn run_SISO(&mut self, input: Vec<f32>) -> Result<ODFormat<Vec<Complex<f32>>>, String> {
        let result = input.iter()
            .map(|x| Complex::new(*x, 0.0))
            .collect();
        
        Ok(ODFormat::Standard(result))
    }
}

pub struct RealCaster {}
impl PipelineStep<Vec<Complex<f32>>, Vec<f32>> for RealCaster {
    fn run_SISO(&mut self, input: Vec<Complex<f32>>) -> Result<ODFormat<Vec<f32>>, String> {
        let result = input.iter()
            .map(|x| x.re)
            .collect();
        
        Ok(ODFormat::Standard(result))
    }   
}