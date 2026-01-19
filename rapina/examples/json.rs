use rapina::extract::{PathParams, FromRequest};
use rapina::prelude::*;

#[derive(Deserialize)]
struct CreateUser {
    name: String,
    email: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

async fn create_user(
    req: hyper::Request<hyper::body::Incoming>,
    params: PathParams,
) -> Json<User> {
    let body = Json::<CreateUser>::from_request(req, &params).await.unwrap();
    let input = body.into_inner();

    Json(User {
        id: 1,
        name: input.name,
        email: input.email,
    })
}

async fn list_users(
    _req: hyper::Request<hyper::body::Incoming>,
    _params: PathParams,
) -> Json<Vec<User>> {
    Json(vec![
        User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@test.com".to_string(),
        },
        User {
            id: 2,
            name: "Bob".to_string(),
            email: "bob@test.com".to_string(),
        },
    ])
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let router = Router::new()
        .get("/users", list_users)
        .post("/users", create_user);

    println!("Endpoints:");
    println!("  GET  /users");
    println!("  POST /users");

    Rapina::new().router(router).listen("127.0.0.1:3000").await
}
