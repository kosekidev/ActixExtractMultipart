use actix_web::{post, App, HttpResponse, HttpServer};
use serde::{Deserialize};
use actix_multipart::Multipart;

use actix_extract_multipart::*;

#[derive(Deserialize)]
struct example {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}

fn saving_file_function(file: File) -> Result<(), ()> {
    // Do some stuff here
    println!("Saving file \"{}\" successfully", file.name);

    Ok(())
}

#[post("/example")]
async fn index(payload: Multipart) -> HttpResponse {
    let example_structure = match extract_multipart::<example>(payload).await {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().json("The data received does not correspond to those expected")
    };
    
    println!("Value of string_param: {}", example_structure.string_param);
    println!("Value of optional_u_param: {:?}", example_structure.optional_u_param);
    println!("Having file? {}", match example_structure.file_param {
        Some(_) => "Yes",
        None => "No"
    });

    if let Some(file) = example_structure.file_param {
        match saving_file_function(file) {
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