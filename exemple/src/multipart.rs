use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize};
use serde_json::{Value, Number, Map};
use std::str;

pub type FileData = Vec<actix_web::web::Bytes>;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileType {
    ImagePNG,
    ImageJPEG,
    ApplicationPDF,
    ApplicationVNDOasisOpendocumentText
}

#[derive(Debug, Deserialize)]
pub struct File {
    pub file_type: FileType,
    pub filename: String,
    pub weight: usize,
    pub path: Option<String>,
}

#[derive(Debug)]
pub struct FileInfos {
    pub file_type: Option<FileType>,
    pub filename: String,
    pub weight: usize,
    pub data: FileData,
}

pub async fn extract_multipart<T>(mut payload: Multipart, images_func: &dyn Fn(FileInfos) -> Option<String>) -> Result<T, Vec<String>>
    // With String: Filename, usize: File weight and the Vec the file data
    where T: serde::de::DeserializeOwned
{
    let mut params = Map::new();
    let mut files_path_uploaded: Vec<String> = Vec::new();

    'mainWhile: while let Ok(Some(mut field)) = payload.try_next().await {
        if let Some(content_disposition) = field.content_disposition() {
            if let Some(field_name) = content_disposition.get_name() {
                if let Some(file_name) = content_disposition.get_filename() {
                    let mut data: FileData = Vec::new();
                    let mut size: usize = 0;

                    while let Some(chunk) = field.next().await {
                        match chunk {
                            Ok(d) => {
                                size += d.len();
                                data.push(d);
                            },
                            Err(_) => {
                                params.insert(field_name.to_owned(), Value::Null);
                                continue 'mainWhile;
                            }
                        }
                    }
            
                    size = (size as f32 / 1.024) as usize; // Convert to real weight

                    if size == 0 {
                        continue 'mainWhile;
                    }

                    let main_type = field.content_type()
                                         .type_()
                                         .to_string()
                                         .replace(".", "")
                                         .replace("_", "")
                                         .replace("-", "");
                    let sub_type = field.content_type()
                                        .subtype()
                                        .to_string()
                                        .replace(".", "")
                                        .replace("_", "")
                                        .replace("-", "");
                    let file_type_str: String = format!("{}{}", main_type, sub_type);
                    let mut sub_params = Map::new();
                    sub_params.insert("file_type".to_owned(), Value::String(file_type_str.clone()));
                    sub_params.insert("filename".to_owned(), Value::String(file_name.to_string()));
                    sub_params.insert("weight".to_owned(), Value::Number(Number::from(size)));

                    let file_type: Option<FileType> = match serde_json::from_value::<FileType>(Value::String(file_type_str)) {
                        Ok(final_type) => Some(final_type),
                        Err(_) => None
                    };

                    match images_func(FileInfos {
                            file_type: file_type,
                            filename: file_name.to_owned(),
                            weight: size,
                            data
                        }) {
                        Some(image_path) => {
                            files_path_uploaded.push(image_path.clone());
                            sub_params.insert("path".to_owned(), Value::String(image_path.to_string()))
                        },
                        None => sub_params.insert("path".to_owned(), Value::Null),
                    };

                    params.insert(field_name.to_owned(), Value::Object(sub_params));
                } else {
                    if let Some(value) = field.next().await {
                        match value {
                            Ok(val) => match str::from_utf8(&val) {
                                Ok(convert_str) => match convert_str.parse::<isize>() {
                                    Ok(number) => params.insert(field_name.to_owned(), Value::Number(Number::from(number))),
                                    Err(_) => match convert_str {
                                        "true" => params.insert(field_name.to_owned(), Value::Bool(true)),
                                        "false" => params.insert(field_name.to_owned(), Value::Bool(false)),
                                        _ => params.insert(field_name.to_owned(), Value::String(convert_str.to_owned()))
                                    },
                                },
                                Err(_) => params.insert(field_name.to_owned(), Value::Null)
                            },
                            Err(_) => params.insert(field_name.to_owned(), Value::Null),
                        };
                    }
                }
            }
        }
    }

    match serde_json::from_value::<T>(Value::Object(params)) {
        Ok(final_struct) => Ok(final_struct),
        Err(_) => Err(files_path_uploaded)
    }
}
