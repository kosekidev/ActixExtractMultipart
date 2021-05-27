use actix_web::{post, App, HttpResponse, HttpServer};
use serde::{Deserialize};
use actix_multipart::Multipart;

mod multipart;
use crate::multipart::*;

#[derive(Deserialize)]
struct Exemple {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}

fn saving_file_function(file_path: &String, _file_data: FileData) -> Result<(), ()> {
    // Do some stuff here
    println!("Saving file \"{}\" successfully", file_path);

    Ok(())
}

fn file_manipulation(file_informations: FileInfos) -> Option<String> {    
    let file_path: String = format!("directory/directory2/{}", &file_informations.filename);
    
    match saving_file_function(&file_path, file_informations.data) {
        Ok(_) => Some(file_path), // Saving success, we return the file path
        Err(_) => None // Saving failed, we return None value
    }
}

#[post("/exemple")]
async fn index(payload: Multipart) -> HttpResponse {
    let exemple_structure = match extract_multipart::<Exemple>(payload, &file_manipulation).await {
        Ok(data) => data,
        Err(files_uploaded) => {
            for file_path in files_uploaded {
                println!("Removing file: {}", file_path);
            }
            return HttpResponse::BadRequest().json("The data received does not correspond to those expected")
        }
    };
    
    println!("Value of string_param: {}", exemple_structure.string_param);
    println!("Value of optional_u_param: {:?}", exemple_structure.optional_u_param);
    println!("Having file? {}", match exemple_structure.file_param {
        Some(_) => "Yes",
        None => "No"
    });

    HttpResponse::Ok().json("Done")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Server run at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}