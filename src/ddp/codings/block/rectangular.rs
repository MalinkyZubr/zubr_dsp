use crate::Pipeline::node::prototype::PipelineStep;
use num::integer::Roots;

fn series_check_parity_stream(stream: &[u8]) -> u8 {
    let mut parity: u8 = 0;
    
    for byte in stream.iter() {
        parity ^= series_check_parity_single(byte);
    }

    parity
}

fn series_check_parity_stream_int(stream: &[u8], vertical_parity: &mut Vec<u8>) -> u8 { // integrated
    let mut parity: u8 = 0;
    
    for (index, byte) in stream.iter().enumerate() {
        parity ^= series_check_parity_single(byte);
        vertical_parity[index] ^= byte;
    }

    parity
}

fn grid_parity_encode(stream: &Vec<u8>, row_length: usize) -> Vec<u8>{
    let adjusted_length: usize = stream.len() + row_length;

    let mut vertical_parity: Vec<u8> = vec![0; (row_length) as usize];
    let mut output: Vec<u8> = Vec::with_capacity(adjusted_length + row_length);

    let mut index = row_length;

    while index < adjusted_length {
        let slice = &stream[(index - row_length)..index];
        let parity_byte = series_check_parity_stream_int(slice, &mut vertical_parity);
        
        output.extend_from_slice(slice);
        output.push(parity_byte);

        index += row_length + 1;
    }

    let vertical_check_parity_byte = series_check_parity_stream(&vertical_parity[0..]);
    output.extend(vertical_parity);
    output.push(vertical_check_parity_byte);

    output
} 

// remember, if a stream of several bytes is even parity, so are all the contained bits

pub fn rectangular_block_encode(input: Vec<u8>) -> Vec<u8> {
    let length = input.len() as usize;
    let row_length: usize = (length).sqrt() as usize;

    if row_length.pow(2) != length { // this should go in the instantiation phase? Dont want this executing every single iteration for high speed engine
        panic!("The unencoded data must have squarable dimension"); // use a result here instead
        let output: Vec<u8> = Vec::new();
        output
    }
    else {
        let output: Vec<u8> = grid_parity_encode(&input, row_length);
        output
    }
}

// fn grid_parity_decode(stream: &Vec<u8>, row_length: usize) -> Vec<u8>{
//     let mut vertical_parity: Vec<u8> = vec![0; (row_length) as usize];
//     let mut horizontal_parity: Vec<u8> = vec![0; (row_length - 1) as usize];
//     let length = stream.len();

//     let mut index = row_length;

//     while index < length {
//         let slice = &stream[(index - row_length)..index];
//         let parity_byte = series_check_parity_stream_int(slice, &mut vertical_parity);
        
//         horizontal_parity.push(parity_byte);

//         index += row_length + 1;
//     }

//     let vertical_check_parity_byte = series_check_parity_stream(&vertical_parity[0..]);
//     output.extend(vertical_parity);

//     output
// } 

// pub fn rectangular_block_decode(input: Vec<u8>) -> Vec<u8> {

// }