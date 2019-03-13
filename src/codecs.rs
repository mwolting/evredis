use quick_error::quick_error;

pub mod resp2;

quick_error! {
    #[derive(Debug)]
    pub enum EncodeError {
        Dummy {}
    }
}

quick_error! {
    #[derive(Debug)]
    pub enum DecodeError {
        Dummy {}
    }
}
