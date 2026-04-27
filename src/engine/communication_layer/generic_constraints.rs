pub trait True {}
impl True for [(); 1] {}


pub trait False {}
impl False for [(); 0] {}