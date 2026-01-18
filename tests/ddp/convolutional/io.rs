#[cfg(test)]
pub mod convolutional_encoder_io {
    use crate::{ddp::codings::convolutional::trellis::{
        encoder_io::{ConvolutionalInputConsumer, ConvolutionalInputProcessor, ConvolutionalOutputByteFactory}, params::{ConvolutionalParams}
    }};

    #[test]
    fn encoder_output_factory_test() {
        {
            let test_params1 = ConvolutionalParams::new(
                8, 
                2, 
                vec![3,5]);
            let mut output_factory: ConvolutionalOutputByteFactory = ConvolutionalOutputByteFactory::new(&test_params1.unwrap());
            let input_stream = vec![3, 3, 3, 3];

            for (idx, value) in input_stream.iter().enumerate() {
                let output = output_factory.append(*value);

                if idx == 3 {
                    dbg!("{}", output.unwrap());
                    assert!(output == Some(255));
                }
                else {
                    assert!(output == None);
                }
            }
        }
        {
            let test_params1 = ConvolutionalParams::new(
                8, 
                4, 
                vec![3,5, 7, 13]);
            let mut output_factory: ConvolutionalOutputByteFactory = ConvolutionalOutputByteFactory::new(&test_params1.unwrap());
            let input_stream = vec![7,2,4,10];

            let mut counter: i32 = 0;
            let correct_values: Vec<u8> = vec![39, 164];

            for value in input_stream.iter() {
                let output = output_factory.append(*value);

                match output {
                    Some(value) => {
                        dbg!("{}", value);
                        let correct_value = correct_values[counter as usize];
                        assert!(output == Some(correct_value));
                        counter += 1;
                    }
                    None => assert!(output == None),
                }
            }
        }
    }

    struct TestProcessor {}

    impl ConvolutionalInputProcessor for TestProcessor {
        fn process(&mut self, input: u8) -> Option<u8> {
            return Some(input);
        }
    }

    #[test]
    fn encoder_input_consumer_test() {
        {
            let test_params1 = ConvolutionalParams::new(
                8, 
                2, 
                vec![3,5]);
            dbg!("{}", &test_params1);
            let mut input_consumer1: ConvolutionalInputConsumer = ConvolutionalInputConsumer::new(
                Box::new(TestProcessor {}), 
                test_params1.unwrap()
            );
            let input_data1 = vec![0b10101010, 0b11110000];
            let output_data1 = input_consumer1.consume(&input_data1);

            assert!(output_data1 == vec![2,2,2,2,0,0,3,3]);
        }
        {
            let test_params = ConvolutionalParams::new(
                8, 
                4, 
                vec![3,5]);
            dbg!("{}", &test_params);
            let mut input_consumer: ConvolutionalInputConsumer = ConvolutionalInputConsumer::new(
                Box::new(TestProcessor {}), 
                test_params.unwrap()
            );
            let input_data = vec![0b10101010, 0b11110000];
            let output_data = input_consumer.consume(&input_data);
    
            assert!(output_data == vec![10, 10, 0, 15]);
        }
    }
}

