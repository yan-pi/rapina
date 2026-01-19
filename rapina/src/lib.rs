pub mod app;
pub mod error;
pub mod extract;
pub mod handler;
pub mod response;
pub mod router;
pub mod server;

pub mod prelude {
    pub use crate::app::Rapina;
    pub use crate::error::{Error, Result};
    pub use crate::extract::{Json, Path};
    pub use crate::response::IntoResponse;
    pub use crate::router::Router;

    pub use http::{Method, StatusCode};
    pub use serde::{Deserialize, Serialize};

    pub use rapina_macros::{delete, get, post, put};
}
