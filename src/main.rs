use actix_web::{web, App, HttpServer, Responder, HttpResponse};
use serde::{Deserialize, Serialize};
use openssl::ssl::{SslAcceptor, SslFiletype, SslMethod};
use std::io::Result;
use actix_files::NamedFile;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct MyObj {
    name: String,
    age: u8,
}

async fn index() -> Result<NamedFile> {
    Ok(NamedFile::open("templates/index.html")?)
}

async fn static_files(req: HttpRequest) -> Result<NamedFile> {
    let path: PathBuf = req.match_info().query("filename").parse().unwrap();
    Ok(NamedFile::open(PathBuf::from("static/").join(path))?)
}

async fn hello() -> impl Responder {
    HttpResponse::Ok().body("Hello, world!")
}

async fn echo(obj: web::Json<MyObj>) -> impl Responder {
    HttpResponse::Ok().json(obj.0)
}

#[actix_web::main]
async fn main() -> Result<()> {
    let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
    builder.set_private_key_file("/etc/letsencrypt/live/dmraise.ru/privkey.pem", SslFiletype::PEM).unwrap();
    builder.set_certificate_chain_file("/etc/letsencrypt/live/dmraise.ru/fullchain.pem").unwrap();

    HttpServer::new(|| {
        App::new()
            .route("/", web::get().to(index))
            .route("/echo", web::post().to(echo))
            .service(actix_files::Files::new("/static", "static").show_files_listing())
    })
    .bind("127.0.0.1:443", builder)?
    .run()
    .await
}
