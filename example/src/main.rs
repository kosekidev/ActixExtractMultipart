use actix_web::{get, post, App, HttpResponse, HttpServer};
use serde::Deserialize;

use actix_extract_multipart::*;
// Accepted files extensions
const FILES_EXTENSIONS: [&str; 2] = ["image/png", "image/jpeg"];

#[derive(Deserialize)]
struct Example {
    string_param: String,
    number_u_param: u32,
    file_param: Option<File>,
}

fn saving_file_function(file: &File) -> Result<(), ()> {
    // Do some stuff here
    println!(
        "Saving file \"{}\" ({} bytes) successfully",
        file.name(),
        file.len()
    );

    Ok(())
}

#[post("/example")]
async fn example_route(payload: Multipart<Example>) -> HttpResponse {
    println!("Value of string_param: {}", &payload.string_param);
    println!("Value of number_u_param: {}", &payload.number_u_param);
    println!(
        "File: {}",
        if payload.file_param.is_some() {
            "YES"
        } else {
            "NO"
        }
    );

    if let Some(file) = &payload.file_param {
        // We getting a file, we can, for example, check file type, saving this file or do some other stuff
        if !FILES_EXTENSIONS.contains(&file.file_type().as_str()) {
            eprintln!("Wrong file format");
            return HttpResponse::BadRequest()
                .json(format!("File's extension must be: {:?}", FILES_EXTENSIONS));
        }

        if saving_file_function(file).is_err() {
            return HttpResponse::InternalServerError().json("");
        };
    }

    HttpResponse::Ok().json("Done")
}
#[get("/")]
async fn index() -> HttpResponse {
    HttpResponse::Ok().body(include_str!("index.html"))
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server run at http://127.0.0.1:8082");

    HttpServer::new(move || App::new().service(index).service(example_route))
        .bind(("127.0.0.1", 8082))?
        .run()
        .await
}
