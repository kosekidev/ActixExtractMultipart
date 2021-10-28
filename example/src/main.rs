use actix_web::{post, App, HttpResponse, HttpServer};
use serde::{Deserialize};

use actix_extract_multipart::*;

#[derive(Deserialize)]
struct Example {
    string_param: String,
    optional_u_param: Option<u32>,
    files_param: Option<Vec<File>>
}

fn saving_files_function(file: &Vec<File>) -> Result<(), ()> {
    // Do some stuff here
    for f in file {
        println!("Saving file \"{}\" successfully (type: {:?})", f.name(), f.file_type());
    }

    Ok(())
}

#[post("/example")]
async fn index(example_structure: Multipart::<Example>) -> HttpResponse {
    println!("Value of string_param: {}", example_structure.string_param);
    println!("Value of optional_u_param: {:?}", example_structure.optional_u_param);
    println!("Having files? {}", match &example_structure.files_param {
        Some(_) => "Yes",
        None => "No"
    });

    if let Some(file) = &example_structure.files_param {
        match saving_files_function(&file) {
            Ok(_) => println!("File saved!"),
            Err(_) => println!("An error occured while file saving")
        }
    }

    HttpResponse::Ok().json("Done")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server run at http://127.0.0.1:8082");

    HttpServer::new(move || {
        App::new()
            .service(index)
    })
    .bind(("127.0.0.1", 8082))?
    .run()
    .await
}