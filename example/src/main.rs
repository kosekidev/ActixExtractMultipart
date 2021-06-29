use actix_web::{post, App, HttpResponse, HttpServer};
use serde::{Deserialize};
use actix_multipart::Multipart;

use actix_extract_multipart::*;

#[derive(Deserialize)]
struct Exemple {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}

fn saving_file_function(file: File) -> Result<(), ()> {
    // Do some stuff here
    println!("Saving file \"{}\" successfully", file.filename);

    Ok(())
}

#[post("/exemple")]
async fn index(payload: Multipart) -> HttpResponse {
    let exemple_structure = match extract_multipart::<Exemple>(payload).await {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().json("The data received does not correspond to those expected")
    };
    
    println!("Value of string_param: {}", exemple_structure.string_param);
    println!("Value of optional_u_param: {:?}", exemple_structure.optional_u_param);
    println!("Having file? {}", match exemple_structure.file_param {
        Some(_) => "Yes",
        None => "No"
    });

    if let Some(file) = exemple_structure.file_param {
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