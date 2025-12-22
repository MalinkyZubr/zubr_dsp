pub mod usb_endpoint_test {
    use rusb;
    use crate::pipeline::endpoints::usb_endpoint::*;
    
    
    #[test]
    fn usb_endpoint_test_implement() {
        let mut counter: i32 = 0;
        for device in rusb::devices().unwrap().iter() {
            let device_desc = device.device_descriptor().unwrap();
            counter += 1;
            println!("Bus {:03} Device {:03} ID {:04x}:{:04x}",
                     device.bus_number(),
                     device.address(),
                     device_desc.vendor_id(),
                     device_desc.product_id());
        }
        
        assert!(counter.is_positive());
    }
}