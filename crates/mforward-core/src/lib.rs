use std::error::Error;

pub type ForwarderError = Box<dyn Error + Send + Sync + 'static>;

pub trait Forwarder {
    type Args;
    fn start(&self, args: &Self::Args) -> impl Future<Output = Result<(), ForwarderError>> + Send;
}
