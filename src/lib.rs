#![crate_name = "actix_extract_multipart"]

use actix_multipart::Multipart;
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize};
use serde_json::{Value, Number, Map};
use std::str;

pub type FileData = Vec<u8>;

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

#[derive(Debug, Deserialize)]
pub struct File {
    file_type: FileType,
    name: String,
    size: u64,
    data: FileData,
}
impl File {
    pub fn file_type(&self) -> &FileType {
        &self.file_type
    }
    pub fn name(&self) -> &String {
        &self.name
    }
    pub fn len(&self) -> u64 {
        self.size
    }
    pub fn data(&self) -> &FileData {
        &self.data
    }
}

fn remove_specials_char(text: String) -> String {
    text.replace(".", "")
        .replace("_", "")
        .replace("-", "")
}
fn get_file_type(content_type: &mime::Mime) -> String {
    let main_type = remove_specials_char(content_type.type_().to_string());
    let sub_type = remove_specials_char(content_type.subtype().to_string());
    format!("{}{}", main_type, sub_type)
}

pub async fn extract_multipart<T>(mut payload: Multipart) -> Result<T, ()>
    where T: serde::de::DeserializeOwned
{
    let mut params = Map::new();

    'mainWhile: while let Ok(Some(mut field)) = payload.try_next().await {
        if let Some(content_disposition) = field.content_disposition() {
            if let Some(field_name) = content_disposition.get_name() {
                if let Some(file_name) = content_disposition.get_filename() {
                    let mut data: Vec<Value> = Vec::new();

                    while let Some(chunk) = field.next().await {
                        match chunk {
                            Ok(d) => {
                                let chunk_data: FileData = d.to_vec();
                                data.reserve_exact(chunk_data.len());
                                for byte in chunk_data {
                                    data.push(Value::Number(Number::from(byte)));
                                }
                            },
                            Err(_) => {
                                params.insert(field_name.to_owned(), Value::Null);
                                continue 'mainWhile;
                            }
                        }
                    }
            
                    let size: usize = data.len();

                    if size == 0 {
                        continue 'mainWhile;
                    }

                    let file_type_str: String = get_file_type(field.content_type());

                    let mut sub_params = Map::new();
                    sub_params.insert("file_type".to_owned(), Value::String(file_type_str.clone()));
                    sub_params.insert("name".to_owned(), Value::String(file_name.to_string()));
                    sub_params.insert("size".to_owned(), Value::Number(Number::from(size)));
                    sub_params.insert("data".to_owned(), Value::Array(data));

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
        Err(_) => Err(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart::Multipart;
    use actix_web::http::header::{self, HeaderMap};
    use tokio::sync::mpsc;
    use actix_web::error::{PayloadError};
    use actix_web::web::Bytes;
    use tokio_stream::wrappers::UnboundedReceiverStream;
    use futures_core::stream::{Stream};
    use serde::{Deserialize};
    use mime;

    fn create_stream() -> (
        mpsc::UnboundedSender<Result<Bytes, PayloadError>>,
        impl Stream<Item = Result<Bytes, PayloadError>>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();

        (
            tx,
            UnboundedReceiverStream::new(rx).map(|res| res.map_err(|_| panic!())),
        )
    }
    fn create_simple_request_with_header() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"file_param\"; filename=\"fn.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"u32_param\"\r\n\r\n\
             56\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"i32_param\"\r\n\r\n\
             -12\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"first_param\"\r\n\r\n\
             A simple test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0--\r\n",
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(
                "multipart/mixed; boundary=\"abbc761f78ff4d7cb7573b5a23f96ef0\"",
            ),
        );
        (bytes, headers)
    }
    fn create_simple_request_with_header_empty_file() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"file_param\"; filename=\"fn.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             \r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"u32_param\"\r\n\r\n\
             56\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"i32_param\"\r\n\r\n\
             -12\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"first_param\"\r\n\r\n\
             A simple test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0--\r\n",
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(
                "multipart/mixed; boundary=\"abbc761f78ff4d7cb7573b5a23f96ef0\"",
            ),
        );
        (bytes, headers)
    }
    fn create_simple_request_with_header_with_no_file() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"u32_param\"\r\n\r\n\
             56\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"i32_param\"\r\n\r\n\
             -12\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"first_param\"\r\n\r\n\
             A simple test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0--\r\n",
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(
                "multipart/mixed; boundary=\"abbc761f78ff4d7cb7573b5a23f96ef0\"",
            ),
        );
        (bytes, headers)
    }
    fn create_simple_request_with_header_with_2_u32() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"param1\"\r\n\r\n\
             56\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"param2\"\r\n\r\n\
             24\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0--\r\n",
        );
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(
                "multipart/mixed; boundary=\"abbc761f78ff4d7cb7573b5a23f96ef0\"",
            ),
        );
        (bytes, headers)
    }

    #[allow(dead_code)]
    #[actix_rt::test]
    async fn test_data_length_after_extraction() {
        #[derive(Deserialize)]
        struct Test {
            first_param: String,
            u32_param: u32,
            i32_param: i32,
            file_param: File,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header();

        sender.send(Ok(bytes)).unwrap();

        let multipart = Multipart::new(&headers, payload);

        match extract_multipart::<Test>(multipart).await {
            Ok(data) => assert_eq!(data.file_param.size, 4),
            Err(_) => panic!("Failed to parse multipart into structure")
        }
    }

    #[allow(dead_code)]
    #[actix_rt::test]
    #[should_panic(expected = "Failed to parse multipart into structure")]
    async fn testing_not_matching_data_types() {
        #[derive(Deserialize)]
        struct Test {
            first_param: String,
            u32_param: i32,
            i32_param: u32,
            file_param: File,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header();

        sender.send(Ok(bytes)).unwrap();

        let multipart = Multipart::new(&headers, payload);

        match extract_multipart::<Test>(multipart).await {
            Ok(_) => panic!("Types not matching, but parsing was a success. It should have return an Err(_)"),
            Err(_) => panic!("Failed to parse multipart into structure")
        }
    }

    #[allow(dead_code)]
    #[actix_rt::test]
    async fn test_empty_file_ignored() {
        #[derive(Deserialize)]
        struct Test {
            first_param: String,
            u32_param: u32,
            i32_param: i32,
            file_param: Option<File>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header_empty_file();

        sender.send(Ok(bytes)).unwrap();

        let multipart = Multipart::new(&headers, payload);

        match extract_multipart::<Test>(multipart).await {
            Ok(data) => assert_eq!(data.file_param.is_none(), true),
            Err(_) => panic!("Failed to parse multipart into structure")
        }
    }

    #[allow(dead_code)]
    #[actix_rt::test]
    async fn test_optional_file_with_file() {
        #[derive(Deserialize)]
        struct Test {
            first_param: String,
            u32_param: u32,
            i32_param: i32,
            file_param: Option<File>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header();

        sender.send(Ok(bytes)).unwrap();

        let multipart = Multipart::new(&headers, payload);

        match extract_multipart::<Test>(multipart).await {
            Ok(data) => assert_eq!(data.file_param.is_some(), true),
            Err(_) => panic!("Failed to parse multipart into structure")
        }
    }

    #[allow(dead_code)]
    #[actix_rt::test]
    async fn test_optional_file_without_file() {
        #[derive(Deserialize)]
        struct Test {
            first_param: String,
            u32_param: u32,
            i32_param: i32,
            file_param: Option<File>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header_with_no_file();

        sender.send(Ok(bytes)).unwrap();

        let multipart = Multipart::new(&headers, payload);

        match extract_multipart::<Test>(multipart).await {
            Ok(data) => assert_eq!(data.file_param.is_none(), true),
            Err(_) => panic!("Failed to parse multipart into structure")
        }
    }

    #[actix_rt::test]
    async fn test_value_in_good_param() {
        #[derive(Deserialize)]
        struct Test {
            param1: u32,
            param2: u32
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header_with_2_u32();

        sender.send(Ok(bytes)).unwrap();

        let multipart = Multipart::new(&headers, payload);

        match extract_multipart::<Test>(multipart).await {
            Ok(data) => assert_eq!(if data.param1 == 56 && data.param2 == 24 { true } else { false }, true),
            Err(_) => panic!("Failed to parse multipart into structure")
        }
    }

    #[actix_rt::test]
    async fn mime_type_to_string() {
        assert_eq!(get_file_type(&mime::APPLICATION_OCTET_STREAM), "applicationoctetstream".to_string())
    }
}
