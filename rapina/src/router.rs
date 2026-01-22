//! HTTP routing for Rapina applications.
//!
//! The [`Router`] type collects route definitions and matches incoming
//! requests to the appropriate handlers.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use http::{Method, Request, Response, StatusCode};
use hyper::body::Incoming;

use crate::extract::{PathParams, extract_path_params};
use crate::response::{BoxBody, IntoResponse};
use crate::state::AppState;

type BoxFuture = Pin<Box<dyn Future<Output = Response<BoxBody>> + Send>>;
type HandlerFn =
    Box<dyn Fn(Request<Incoming>, PathParams, Arc<AppState>) -> BoxFuture + Send + Sync>;

pub(crate) struct Route {
    pub(crate) pattern: String,
    handler: HandlerFn,
}

/// The HTTP router for matching requests to handlers.
///
/// Routes are matched in the order they are added. Use path parameters
/// with the `:param` syntax.
///
/// # Examples
///
/// ```
/// use rapina::prelude::*;
///
/// let router = Router::new()
///     .get("/", |_, _, _| async { "Hello!" })
///     .get("/users/:id", |_, _, _| async { "User" })
///     .post("/users", |_, _, _| async { StatusCode::CREATED });
/// ```
pub struct Router {
    pub(crate) routes: Vec<(Method, Route)>,
}

impl Router {
    /// Creates a new empty router.
    pub fn new() -> Self {
        Self { routes: Vec::new() }
    }

    /// Adds a route with the given HTTP method and pattern.
    pub fn route<F, Fut, Out>(mut self, method: Method, pattern: &str, handler: F) -> Self
    where
        F: Fn(Request<Incoming>, PathParams, Arc<AppState>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Out> + Send + 'static,
        Out: IntoResponse + 'static,
    {
        let handler = Box::new(
            move |req: Request<Incoming>, params: PathParams, state: Arc<AppState>| {
                let handler = handler.clone();
                Box::pin(async move {
                    let output = handler(req, params, state).await;
                    output.into_response()
                }) as BoxFuture
            },
        );

        let route = Route {
            pattern: pattern.to_string(),
            handler,
        };

        self.routes.push((method, route));
        self
    }

    /// Adds a GET route.
    pub fn get<F, Fut, Out>(self, pattern: &str, handler: F) -> Self
    where
        F: Fn(Request<Incoming>, PathParams, Arc<AppState>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Out> + Send + 'static,
        Out: IntoResponse + 'static,
    {
        self.route(Method::GET, pattern, handler)
    }

    /// Adds a POST route.
    pub fn post<F, Fut, Out>(self, pattern: &str, handler: F) -> Self
    where
        F: Fn(Request<Incoming>, PathParams, Arc<AppState>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = Out> + Send + 'static,
        Out: IntoResponse + 'static,
    {
        self.route(Method::POST, pattern, handler)
    }

    /// Handles an incoming request by matching it to a route.
    pub async fn handle(&self, req: Request<Incoming>, state: &Arc<AppState>) -> Response<BoxBody> {
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        for (route_method, route) in &self.routes {
            if *route_method != method {
                continue;
            }

            if let Some(params) = extract_path_params(&route.pattern, &path) {
                return (route.handler)(req, params, state.clone()).await;
            }
        }

        StatusCode::NOT_FOUND.into_response()
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_router_new() {
        let router = Router::new();
        assert!(router.routes.is_empty());
    }

    #[test]
    fn test_router_default() {
        let router = Router::default();
        assert!(router.routes.is_empty());
    }

    #[test]
    fn test_router_add_get_route() {
        let router = Router::new().get("/users", |_req, _params, _state| async { StatusCode::OK });
        assert_eq!(router.routes.len(), 1);
        assert_eq!(router.routes[0].0, Method::GET);
        assert_eq!(router.routes[0].1.pattern, "/users");
    }

    #[test]
    fn test_router_add_post_route() {
        let router = Router::new().post("/users", |_req, _params, _state| async {
            StatusCode::CREATED
        });
        assert_eq!(router.routes.len(), 1);
        assert_eq!(router.routes[0].0, Method::POST);
        assert_eq!(router.routes[0].1.pattern, "/users");
    }

    #[test]
    fn test_router_add_custom_method_route() {
        let router =
            Router::new().route(Method::PUT, "/users/:id", |_req, _params, _state| async {
                StatusCode::OK
            });
        assert_eq!(router.routes.len(), 1);
        assert_eq!(router.routes[0].0, Method::PUT);
        assert_eq!(router.routes[0].1.pattern, "/users/:id");
    }

    #[test]
    fn test_router_multiple_routes() {
        let router = Router::new()
            .get("/users", |_req, _params, _state| async { StatusCode::OK })
            .post("/users", |_req, _params, _state| async {
                StatusCode::CREATED
            })
            .route(
                Method::DELETE,
                "/users/:id",
                |_req, _params, _state| async { StatusCode::NO_CONTENT },
            );

        assert_eq!(router.routes.len(), 3);
        assert_eq!(router.routes[0].0, Method::GET);
        assert_eq!(router.routes[1].0, Method::POST);
        assert_eq!(router.routes[2].0, Method::DELETE);
    }

    #[test]
    fn test_router_chaining() {
        let router = Router::new()
            .get("/", |_req, _params, _state| async { StatusCode::OK })
            .get("/health", |_req, _params, _state| async { StatusCode::OK });

        assert_eq!(router.routes.len(), 2);
    }

    #[test]
    fn test_router_preserves_route_order() {
        let router = Router::new()
            .get("/first", |_req, _params, _state| async { StatusCode::OK })
            .get("/second", |_req, _params, _state| async { StatusCode::OK })
            .get("/third", |_req, _params, _state| async { StatusCode::OK });

        assert_eq!(router.routes[0].1.pattern, "/first");
        assert_eq!(router.routes[1].1.pattern, "/second");
        assert_eq!(router.routes[2].1.pattern, "/third");
    }
}
