use std::error::Error;

pub type ForwarderError = Box<dyn Error + Send + Sync + 'static>;

pub trait Forwarder {
    type A;
    fn start(&self, args: &Self::A) -> impl Future<Output = Result<(), ForwarderError>> + Send;
}
