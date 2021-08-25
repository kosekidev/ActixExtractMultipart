# ActixExtractMultipart
Functions and structures to handle actix multipart more easily. You can convert the multipart into a struct.

To use this function, you need to create a structure with "Deserialize" trait, like this:
```rust
#[derive(Deserialize)]
struct example {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}
```
File is a structure for any files:
```rust
#[derive(Debug, Deserialize)]
pub struct File {
    pub #type: FileType,
    pub name: String,
    size: usize,
    pub data: FileData,
}
impl File {
    // Return the size of the file
    pub fn len(&self) -> u64 {
        self.size
    }
}
```
FileData is an alias to Vec<u8> bytes: (Defined in multipart.rs file)
```rust
pub type FileData = Vec<u8>;
```
Then, we can call the extract_multipart function. It takes in parameter the actix_multipart::Multipart and precise the structure output like this:
    
```rust
async fn extract_multipart::<StructureType: T>(actix_mutlipart::Multipart) -> Result<T, _>
```

## Example of use
```rust
use actix_web::{post, App, HttpResponse, HttpServer};
use serde::{Deserialize};
use actix_multipart::Multipart;

use actix_extract_multipart::*;

#[derive(Deserialize)]
struct example {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: File
}

fn saving_file_function(file: File) -> Result<(), ()> {
    // Do some stuff here
    println!("Saving file \"{}\" successfully", file.filename);

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
    println!("Server run at http://127.0.0.1:8080");

    HttpServer::new(move || {
        App::new()
            .service(index)
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
```
In this example, if you dont have received a file, extract_multipart will return an Err(_), because data don't correspond to the data struct "example".
If the File is optional, you can simply set the type as Option<File>, like this:
```rust
#[derive(Deserialize)]
struct example {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}
```
The function extract_multipart will return Err(_) value also if the file type was not in FileType enumeration.
```
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    ImagePNG,
    ImageJPEG,
    ImageGIF,
    ImageWEBP,
    ApplicationPDF,
    ApplicationJSON,
    ApplicationXML,
    TextCSV,
    TextPlain,
    #[serde(alias = "applicationvndoasisopendocumenttext")]
    ODT,
    #[serde(alias = "applicationvndoasisopendocumentspreadsheet")]
    ODS,
    #[serde(alias = "applicationvndmsexcel")]
    XLS,
    #[serde(alias = "applicationvndopenxmlformatsofficedocumentspreadsheetmlsheet")]
    XLSX,
}
```
