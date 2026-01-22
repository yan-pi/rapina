mod body_limit;
mod request_log;
mod timeout;
mod trace_id;

pub use body_limit::BodyLimitMiddleware;
pub use request_log::RequestLogMiddleware;
pub use timeout::TimeoutMiddleware;
pub use trace_id::TraceIdMiddleware;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use hyper::body::Incoming;
use hyper::{Request, Response};

use crate::context::RequestContext;
use crate::response::BoxBody;
use crate::router::Router;
use crate::state::AppState;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait Middleware: Send + Sync + 'static {
    fn handle<'a>(
        &'a self,
        req: Request<Incoming>,
        ctx: &'a RequestContext,
        next: Next<'a>,
    ) -> BoxFuture<'a, Response<BoxBody>>;
}

pub struct Next<'a> {
    middlewares: &'a [Arc<dyn Middleware>],
    router: &'a Router,
    state: &'a Arc<AppState>,
    ctx: &'a RequestContext,
}

impl<'a> Next<'a> {
    pub(crate) fn new(
        middlewares: &'a [Arc<dyn Middleware>],
        router: &'a Router,
        state: &'a Arc<AppState>,
        ctx: &'a RequestContext,
    ) -> Self {
        Self {
            middlewares,
            router,
            state,
            ctx,
        }
    }

    pub async fn run(self, req: Request<Incoming>) -> Response<BoxBody> {
        if let Some((current, rest)) = self.middlewares.split_first() {
            let next = Next {
                middlewares: rest,
                router: self.router,
                state: self.state,
                ctx: self.ctx,
            };
            current.handle(req, self.ctx, next).await
        } else {
            self.router.handle(req, self.state).await
        }
    }
}

pub struct MiddlewareStack {
    middlewares: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareStack {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }

    pub fn add<M: Middleware>(&mut self, middleware: M) {
        self.middlewares.push(Arc::new(middleware));
    }

    pub fn push(&mut self, middleware: Arc<dyn Middleware>) {
        self.middlewares.push(middleware);
    }

    pub async fn execute(
        &self,
        req: Request<Incoming>,
        router: &Router,
        state: &Arc<AppState>,
        ctx: &RequestContext,
    ) -> Response<BoxBody> {
        let next = Next::new(&self.middlewares, router, state, ctx);
        next.run(req).await
    }

    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // Test middleware for testing purposes
    struct TestMiddleware;

    impl Middleware for TestMiddleware {
        fn handle<'a>(
            &'a self,
            req: Request<Incoming>,
            _ctx: &'a RequestContext,
            next: Next<'a>,
        ) -> BoxFuture<'a, Response<BoxBody>> {
            Box::pin(async move { next.run(req).await })
        }
    }

    #[test]
    fn test_middleware_stack_new() {
        let stack = MiddlewareStack::new();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_middleware_stack_default() {
        let stack = MiddlewareStack::default();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_middleware_stack_add() {
        let mut stack = MiddlewareStack::new();
        stack.add(TestMiddleware);
        assert!(!stack.is_empty());
        assert_eq!(stack.middlewares.len(), 1);
    }

    #[test]
    fn test_middleware_stack_push() {
        let mut stack = MiddlewareStack::new();
        stack.push(Arc::new(TestMiddleware));
        assert!(!stack.is_empty());
        assert_eq!(stack.middlewares.len(), 1);
    }

    #[test]
    fn test_middleware_stack_multiple() {
        let mut stack = MiddlewareStack::new();
        stack.add(TestMiddleware);
        stack.add(TestMiddleware);
        stack.push(Arc::new(TestMiddleware));
        assert_eq!(stack.middlewares.len(), 3);
    }

    #[test]
    fn test_timeout_middleware_new() {
        let mw = TimeoutMiddleware::new(Duration::from_secs(60));
        assert_eq!(mw.duration, Duration::from_secs(60));
    }

    #[test]
    fn test_timeout_middleware_default() {
        let mw = TimeoutMiddleware::default();
        assert_eq!(mw.duration, Duration::from_secs(30));
    }

    #[test]
    fn test_body_limit_middleware_new() {
        let mw = BodyLimitMiddleware::new(2048);
        assert_eq!(mw.max_size, 2048);
    }

    #[test]
    fn test_body_limit_middleware_default() {
        let mw = BodyLimitMiddleware::default();
        assert_eq!(mw.max_size, 1024 * 1024); // 1MB default
    }

    #[test]
    fn test_trace_id_middleware_new() {
        let _mw = TraceIdMiddleware::new();
    }

    #[test]
    fn test_trace_id_middleware_default() {
        let _mw: TraceIdMiddleware = Default::default();
    }
}
