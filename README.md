[![No Maintenance Intended](http://unmaintained.tech/badge.svg)](http://unmaintained.tech/)
> **Warning**
> This crate will be no longer maintained
> See : https://crates.io/crates/actix-form-data

# ActixExtractMultipart
Functions and structures to handle actix multipart more easily. You can convert the multipart into a struct.

To use this function, you need to create a structure with "Deserialize" trait, like this:
```rust
#[derive(Deserialize)]
struct Example {
    string_param: String,
    optional_u_param: Option<u32>,
    files_param: Option<Vec<File>>
}
```
File is a structure for any files:
```rust
#[derive(Debug, Deserialize)]
pub struct File {
    file_type: String,
    name: String,
    data: FileData,
}
impl File {
    pub fn file_type(&self) -> &String {
        &self.file_type
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }
    pub fn data(&self) -> &FileData {
        &self.data
    }
}
```
FileData is an alias to Vec<u8> bytes:
```rust
pub type FileData = Vec<u8>;
```

## Example of use
```rust
use actix_web::{post, App, HttpResponse, HttpServer};
use serde::{Deserialize};
use actix_extract_multipart::*;

#[derive(Deserialize)]
struct Example {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: File
}

fn saving_file_function(file: &File) -> Result<(), ()> {
    // Do some stuff here
    println!("Saving file \"{}\" successfully", file.name());

    Ok(())
}

#[post("/example")]
async fn index(example_structure: Multipart::<Example>) -> HttpResponse {    
    println!("Value of string_param: {}", example_structure.string_param);
    println!("Value of optional_u_param: {:?}", example_structure.optional_u_param);
    println!("Having file? {}", match example_structure.file_param {
        Some(_) => "Yes",
        None => "No"
    });

    if let Some(file) = &example_structure.file_param {
        match saving_file_function(&file) {
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
In this example, if you dont have received a file, extract_multipart will return an Err(_), because data don't correspond to the data struct "Example".
If the File is optional, you can simply set the type as Option<File>, like this:
```rust
#[derive(Deserialize)]
struct Example {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}
```
In the case of Vec<File>, don't forget to put hooks at the end of the field name. You can also have any other type array like Vec<String>, Vec<i32> etc...
In the follow html exemple, you can notice that the file's field's name contain hooks: name="files_param[]".
It's important, without hooks, this code will not work.
```html
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="X-UA-Compatible" content="IE=edge">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Testing</title>
    <script>
        function send() {
            let myHeaders = new Headers();
            let formdata = new FormData(document.getElementById('form'));
            let myInit = { method: 'POST', headers: myHeaders, body: formdata };
            fetch("http://127.0.0.1:8082/example", myInit)
            .then(() => {
                console.log("It works!")
            })
            .catch((e) => {
                console.log("Error!\n" + e)
            })
        }
    </script>
</head>
<body>
    <form id="form">
        <input type="file" name="files_param[]" multiple>
        <button type="button" onclick="send()">OK</button>
    </form>
</body>
</html>
```
