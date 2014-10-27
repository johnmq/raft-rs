pub trait Intercommunication {
    fn new() -> Self;
}

pub struct DefaultIntercommunication;

impl Intercommunication for DefaultIntercommunication {
    fn new() -> DefaultIntercommunication {
        DefaultIntercommunication
    }
}

