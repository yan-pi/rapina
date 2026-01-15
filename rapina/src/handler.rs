use std::future::Future;

use crate::response::IntoResponse;

pub trait Handler<Args>: Clone + Send + 'static {
    type Output: IntoResponse;
    type Future: Future<Output = Self::Output> + Send;

    fn call(self, args: Args) -> Self::Future;
}

impl<F, Fut, Out> Handler<()> for F
where
    F: FnOnce() -> Fut + Clone + Send + 'static,
    Fut: Future<Output = Out> + Send,
    Out: IntoResponse,
{
    type Output = Out;
    type Future = Fut;

    fn call(self, _args: ()) -> Self::Future {
        (self)()
    }
}
