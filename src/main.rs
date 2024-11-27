// actix_webはrustでのapiなどwebアプリ作成に必要な機能を追加する。pythonのfastapiのようなもの
use actix_web::{get, post, web, App, HttpResponse, HttpServer};
// askama templateはrustとhtmlを繋げる。rustのデータをhtmlに埋め込んだりする
use askama::Template;
// actix_webとaskamaを連携させるための機能, askamaテンプレートをhttpresponseに変換してhtmlを返すエンドポイントを簡単に作れる
use askama_actix::TemplateToResponse;
// sqlightのこと
use sqlx::{Row, SqlitePool};

#[derive(Template)]
#[template(path = "hello.html")]
struct HelloTemplate {
    name: String,
}

#[derive(Template)]
#[template(path = "todo.html")]
struct TodoTemplate {
    tasks: Vec<String>,
}

#[derive(serde::Deserialize)]
struct Task {
    // Option型にすることでどちらかだけ入力されたときでも問題なく動作する
    id: Option<String>,
    task: Option<String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // データベースとの接続
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    sqlx::query("CREATE TABLE tasks (task TEXT);")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tasks (task) VALUES ('タスク1');")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tasks (task) VALUES ('タスク2');")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO tasks (task) VALUES ('タスク3');")
        .execute(&pool)
        .await
        .unwrap();

    // moveはpoolのcloneをどこからでもアクセスできるようにHttpserverに無名関数として所有権ごと渡している
    HttpServer::new(move || {
        App::new()
            .service(hello_world)
            .service(todo)
            .app_data(web::Data::new(pool.clone()))
            .service(update)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}

#[get("/hello/{name}")]
async fn hello_world(name: web::Path<String>) -> HttpResponse {
    let hello = HelloTemplate {
        // 値を取り出している。 web::Path<String> -> String
        name: name.into_inner(),
    };
    hello.to_response()
}

#[get("/todo")]
async fn todo(pool: web::Data<SqlitePool>) -> HttpResponse {
    let rows = sqlx::query("SELECT task FROM tasks;")
        .fetch_all(pool.as_ref())
        .await
        .unwrap();
    let tasks = rows
        .iter()
        .map(|row| row.get::<String, _>("task"))
        .collect();
    let todo = TodoTemplate { tasks };
    todo.to_response()
}

#[post("/update")]
async fn update(pool: web::Data<SqlitePool>, form: web::Form<Task>) -> HttpResponse {
    let task = form.into_inner();
    if let Some(id) = task.id {
        sqlx::query("DELETE FROM tasks WHERE task = ?")
            .bind(id)
            .execute(pool.as_ref())
            .await
            .unwrap();
    }
    match task.task {
        Some(task) if !task.is_empty() => {
            sqlx::query("INSERT INTO tasks (task) VALUES (?)")
                .bind(task)
                .execute(pool.as_ref()) // SqlitePool -> !SqlitePoolに変換(所有権を渡さずにpoolを利用), &poolよりも柔軟性があり、明確に直接参照型でなくてもAsRef<T>トレイトを実装していれば参照を取得できる
                .await
                .unwrap();
        }
        _ => {}
    }
    HttpResponse::Found()
        .append_header(("Location", "/todo"))
        .finish()
}
