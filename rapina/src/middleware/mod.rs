mod body_limit;
mod timeout;
mod trace_id;

pub use body_limit::BodyLimitMiddleware;
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
