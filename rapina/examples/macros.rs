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

#[get("/")]
async fn hello() -> &'static str {
    "Hello, Rapina!"
}

#[get("/health")]
async fn health() -> StatusCode {
    StatusCode::OK
}

#[get("/users/:id")]
async fn get_user(id: Path<u64>) -> String {
    format!("User ID: {}", id.into_inner())
}

#[post("/users")]
async fn create_user(body: Json<CreateUser>) -> Json<User> {
    let input = body.into_inner();
    Json(User {
        id: 1,
        name: input.name,
        email: input.email,
    })
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let router = Router::new()
        .get("/", hello)
        .get("/health", health)
        .get("/users/:id", get_user)
        .post("/users", create_user);

    Rapina::new().router(router).listen("127.0.0.1:3000").await
}
