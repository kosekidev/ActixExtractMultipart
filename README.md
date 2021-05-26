# ActixExtractMultipart
Functions and structures to handle actix multipart more easily. You can convert the multipart into a struct and do some stuff on image data(Compression, saving etc...)

The function extract_multipart have 2 argument:
First, the actix multipart and a function.
The function is called for each file received. You can therefore compress, modify and/or save your files as well as cancel the processing thereof in the event of violation of some of your constraints (file size, wrong file type etc ...).
Theses functions need to have this signature:
```rust
Fn(&str, usize, FileData) -> Option<String>
```
The first parameter(&str) is the name of the file.
The second parameter(usize) is the file weight.
The third parameter(FileData) is the file data.

FileData is an alias to vec actixweb bytes: (Defined in multipart.rs file)
```rust
pub type FileData = Vec<actix_web::web::Bytes>;
```
Function must be return the file path(Option<String>), needed by File structure. If you return None value, the path parameter of File structure be setting on None.
```rust
#[derive(Debug, Deserialize)]
pub struct File {
    pub file_type: FileType,
    pub filename: String,
    pub weight: usize,
    pub path: Option<String>,
}
```

## Example of use
```rust
use serde::{Deserialize};
use actix_multipart::Multipart; // Actix multipart
use crate::multipart::; // The multipart.rs file

#[derive(Deserialize)]
struct Exemple {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: File
}

fn file_manipulation(filename: &str, file_weight: usize, file_data: FileData) -> Option<String> {
    // Here, we can do some stuff with the file data
    // FileData type is an alias of Vec<actix_web::web::Bytes>
    
    let file_data_compressed = compression_function(file_data);
    let file_path: String = "directory/directory2/".to_owned();
    
    match saving_file_function(file_path, file_data_compressed) {
      Ok(_) => Some(file_path), // Saving success, we return the file path
      Err(_) => None // Saving failed, we return None value
    }
}

#[post("/exemple")]
async fn exemple_func(payload: Multipart) -> HttpResponse {
    let exemple_structure = match extract_multipart::<Exemple>(payload, &file_manipulation).await {
        Some(data) => data,
        None => return HttpResponse::BadRequest().json("The data received does not correspond to those expected")
    };
    
    println!("Value of string_param: {}", exemple_structure.string_param);

    HttpResponse::Ok().json("Done")
}
```
In this exemple, if you dont have received a file, extract_multipart will return None value, because data don't correspond to the data struct Exemple.
If the File is optional, you can simply set the type as Option<File> like this:
```rust
#[derive(Deserialize)]
struct Exemple {
    string_param: String,
    optional_u_param: Option<u32>,
    file_param: Option<File>
}
```
The function extract_multipart will return None value also if the file type was not in FileType enumeration.
```
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    ImagePNG,
    ImageJPEG,
    ApplicationPDF,
}
```
You can add types in this enumeration if needed.
FileType was made width mime::Mime crate:
```rust
let file_type = format!("{}{}", field.content_type().type_(), field.content_type().subtype());
```
We just concat the (mime::Mime).type_() return and the (mime::Mime).subtype() value to make our type.
  
For exemple:
  
You want accept .gif images.
The value returned by (mime::Image_GIF).type_() is **"image"** and the value returned by (mime::Image_GIF).subtype() is **"png"**.
The filetype generated is therefore: **"imagegif**.
So, for accept .gif images, you just have to add "ImageGIF" to the FileType structure:
```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    ImagePNG,
    ImageJPEG,
    ImageGIF,
    ApplicationPDF,
}
```
