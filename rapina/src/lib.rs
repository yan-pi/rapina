pub mod app;
pub mod context;
pub mod error;
pub mod extract;
pub mod handler;
pub mod middleware;
pub mod response;
pub mod router;
pub mod server;
pub mod state;

pub mod prelude {
    pub use crate::app::Rapina;
    pub use crate::context::RequestContext;
    pub use crate::error::{Error, Result};
    pub use crate::extract::{Context, Json, Path, Query};
    pub use crate::middleware::{Middleware, Next};
    pub use crate::response::IntoResponse;
    pub use crate::router::Router;

    pub use http::{Method, StatusCode};
    pub use serde::{Deserialize, Serialize};

    pub use rapina_macros::{delete, get, post, put};
}
