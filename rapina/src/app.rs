//! The main application builder for Rapina.

use std::net::SocketAddr;

use crate::middleware::{Middleware, MiddlewareStack};
use crate::observability::TracingConfig;
use crate::router::Router;
use crate::server::serve;
use crate::state::AppState;

/// The main application type for building Rapina servers.
///
/// Use the builder pattern to configure routing, state, middleware,
/// and observability before starting the server.
///
/// # Examples
///
/// ```rust,no_run
/// use rapina::prelude::*;
///
/// #[tokio::main]
/// async fn main() -> std::io::Result<()> {
///     let router = Router::new()
///         .get("/", |_, _, _| async { "Hello!" });
///
///     Rapina::new()
///         .router(router)
///         .listen("127.0.0.1:3000")
///         .await
/// }
/// ```
pub struct Rapina {
    router: Router,
    state: AppState,
    middlewares: MiddlewareStack,
}

impl Rapina {
    /// Creates a new Rapina application builder.
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            state: AppState::new(),
            middlewares: MiddlewareStack::new(),
        }
    }

    /// Sets the router for the application.
    pub fn router(mut self, router: Router) -> Self {
        self.router = router;
        self
    }

    /// Adds shared state that can be accessed by handlers via [`State`](crate::extract::State).
    pub fn state<T: Send + Sync + 'static>(mut self, value: T) -> Self {
        self.state = self.state.with(value);
        self
    }

    /// Adds a middleware to the application.
    pub fn middleware<M: Middleware>(mut self, middleware: M) -> Self {
        self.middlewares.add(middleware);
        self
    }

    /// Configures tracing/logging for the application.
    pub fn with_tracing(self, config: TracingConfig) -> Self {
        config.init();
        self
    }

    /// Starts the HTTP server on the given address.
    ///
    /// # Panics
    ///
    /// Panics if the address cannot be parsed.
    pub async fn listen(self, addr: &str) -> std::io::Result<()> {
        let addr: SocketAddr = addr.parse().expect("invalid address");
        serve(self.router, self.state, self.middlewares, addr).await
    }
}

impl Default for Rapina {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware::TimeoutMiddleware;
    use http::StatusCode;
    use std::time::Duration;

    #[test]
    fn test_rapina_new() {
        let app = Rapina::new();
        assert!(app.middlewares.is_empty());
    }

    #[test]
    fn test_rapina_default() {
        let app = Rapina::default();
        assert!(app.middlewares.is_empty());
    }

    #[test]
    fn test_rapina_with_router() {
        let router = Router::new().get("/health", |_req, _params, _state| async { StatusCode::OK });

        let app = Rapina::new().router(router);
        assert!(!app.router.routes.is_empty());
    }

    #[test]
    fn test_rapina_with_state() {
        #[derive(Clone)]
        struct Config {
            name: String,
        }

        let app = Rapina::new().state(Config {
            name: "test".to_string(),
        });

        let config = app.state.get::<Config>().unwrap();
        assert_eq!(config.name, "test");
    }

    #[test]
    fn test_rapina_with_middleware() {
        let app = Rapina::new().middleware(TimeoutMiddleware::new(Duration::from_secs(10)));

        assert!(!app.middlewares.is_empty());
    }

    #[test]
    fn test_rapina_chaining() {
        #[allow(dead_code)]
        #[derive(Clone)]
        struct Config {
            name: String,
        }

        let router = Router::new()
            .get("/", |_req, _params, _state| async { StatusCode::OK })
            .post("/users", |_req, _params, _state| async {
                StatusCode::CREATED
            });

        let app = Rapina::new()
            .router(router)
            .state(Config {
                name: "app".to_string(),
            })
            .middleware(TimeoutMiddleware::default());

        assert!(!app.router.routes.is_empty());
        assert!(app.state.get::<Config>().is_some());
        assert!(!app.middlewares.is_empty());
    }

    #[test]
    fn test_rapina_multiple_states() {
        #[allow(dead_code)]
        #[derive(Clone)]
        struct Config {
            name: String,
        }

        #[allow(dead_code)]
        #[derive(Clone)]
        struct DbPool {
            url: String,
        }

        let app = Rapina::new()
            .state(Config {
                name: "app".to_string(),
            })
            .state(DbPool {
                url: "postgres://localhost".to_string(),
            });

        assert!(app.state.get::<Config>().is_some());
        assert!(app.state.get::<DbPool>().is_some());
    }

    #[test]
    fn test_rapina_multiple_middlewares() {
        use crate::middleware::{BodyLimitMiddleware, TraceIdMiddleware};

        let app = Rapina::new()
            .middleware(TraceIdMiddleware::new())
            .middleware(TimeoutMiddleware::default())
            .middleware(BodyLimitMiddleware::default());

        // MiddlewareStack doesn't expose count, but we can verify it's not empty
        assert!(!app.middlewares.is_empty());
    }
}
