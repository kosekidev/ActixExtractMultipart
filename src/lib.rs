#![crate_name = "actix_extract_multipart"]

use actix_multipart;
use futures::{StreamExt, TryStreamExt};
use serde::Deserialize;
use serde_json::{Map, Number, Value};
use std::ops::{Deref, DerefMut};
use std::str;

use actix_web::{dev::Payload, Error, FromRequest, HttpRequest};
use futures_util::future::Future;
use std::pin::Pin;

pub type FileData = Vec<u8>;

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

fn params_insert(
    params: &mut Map<String, Value>,
    field_name: &str,
    field_name_formatted: &String,
    element: Value,
) {
    if params.contains_key(field_name_formatted) {
        if let Value::Array(val) = params.get_mut(field_name_formatted).unwrap() {
            val.push(element);
        }
    } else if field_name.ends_with("[]") {
        params.insert(field_name_formatted.to_owned(), Value::Array(vec![element]));
    } else {
        params.insert(field_name_formatted.to_owned(), element);
    }
}

pub struct Multipart<T> {
    data: T,
}

impl<T> Multipart<T> {
    fn new(data: T) -> Self {
        Multipart::<T> { data }
    }
}

impl<T> Deref for Multipart<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.data
    }
}

impl<T> DerefMut for Multipart<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.data
    }
}

async fn extract_multipart<T>(
    mut payload: actix_multipart::Multipart,
) -> Result<T, serde_json::Error>
where
    T: serde::de::DeserializeOwned,
{
    let mut params = Map::new();

    'mainWhile: while let Ok(Some(mut field)) = payload.try_next().await {
        let field_name_string = field.content_disposition().get_name().unwrap().to_string();
        let field_name = field_name_string.as_str();
        let field_name_formatted = field_name.replace("[]", "");

        if field.content_disposition().get_filename().is_some() {
            let mut data: Vec<Value> = Vec::new();
            let file_name = field
                .content_disposition()
                .get_filename()
                .unwrap()
                .to_string();

            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(d) => {
                        let chunk_data: FileData = d.to_vec();
                        data.reserve_exact(chunk_data.len());
                        for byte in chunk_data {
                            data.push(Value::Number(Number::from(byte)));
                        }
                    }
                    Err(_) => {
                        params.insert(field_name_formatted.to_owned(), Value::Null);
                        continue 'mainWhile;
                    }
                }
            }
            if data.is_empty() {
                continue 'mainWhile;
            }

            let file_type_str: String = field.content_type().to_string();

            let mut sub_params = Map::new();
            sub_params.insert("file_type".to_owned(), Value::String(file_type_str));
            sub_params.insert("name".to_owned(), Value::String(file_name));
            sub_params.insert("data".to_owned(), Value::Array(data));

            params_insert(
                &mut params,
                field_name,
                &field_name_formatted,
                Value::Object(sub_params),
            );
        } else if let Some(value) = field.next().await {
            if let Ok(val) = value {
                if let Ok(convert_str) = str::from_utf8(&val) {
                    match convert_str.parse::<isize>() {
                        Ok(number) => params_insert(
                            &mut params,
                            field_name,
                            &field_name_formatted,
                            Value::Number(Number::from(number)),
                        ),
                        Err(_) => match convert_str {
                            "true" => params_insert(
                                &mut params,
                                field_name,
                                &field_name_formatted,
                                Value::Bool(true),
                            ),
                            "false" => params_insert(
                                &mut params,
                                field_name,
                                &field_name_formatted,
                                Value::Bool(false),
                            ),
                            _ => params_insert(
                                &mut params,
                                field_name,
                                &field_name_formatted,
                                Value::String(convert_str.to_owned()),
                            ),
                        },
                    }
                }
                continue 'mainWhile;
            }

            params_insert(&mut params, field_name, &field_name_formatted, Value::Null)
        }
    }

    serde_json::from_value::<T>(Value::Object(params))
}

impl<T: serde::de::DeserializeOwned> FromRequest for Multipart<T> {
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self, Self::Error>>>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let multipart = actix_multipart::Multipart::new(req.headers(), payload.take());

        Box::pin(async move {
            match extract_multipart::<T>(multipart).await {
                Ok(response) => Ok(Multipart::<T>::new(response)),
                Err(_) => Err(actix_web::error::ErrorBadRequest("")),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_multipart;
    use actix_web::error::PayloadError;
    use actix_web::http::header::{self, HeaderMap};
    use actix_web::web::Bytes;
    use futures_core::stream::Stream;
    use serde::Deserialize;
    use tokio::sync::mpsc;
    use tokio_stream::wrappers::UnboundedReceiverStream;

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
    fn create_simple_request_with_array_header() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"u32_param[]\"\r\n\r\n\
             56\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"u32_param[]\"\r\n\r\n\
             49\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"i32_param[]\"\r\n\r\n\
             -12\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"i32_param[]\"\r\n\r\n\
             -2\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"i32_param[]\"\r\n\r\n\
             -17\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"string_param[]\"\r\n\r\n\
             A simple test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"string_param[]\"\r\n\r\n\
             A simple test2\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"string_param[]\"\r\n\r\n\
             A simple test3\r\n\
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
    fn create_simple_request_with_3_files_array_header() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param[]\"; filename=\"fn.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param[]\"; filename=\"fn2.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param[]\"; filename=\"fn3.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
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
    fn create_simple_request_with_1_files_array_header() -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param[]\"; filename=\"fn.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
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
    fn create_simple_request_with_3_files_array_with_name_without_hooks_header(
    ) -> (Bytes, HeaderMap) {
        let bytes = Bytes::from(
            "testasdadsad\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param\"; filename=\"fn.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param\"; filename=\"fn2txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
             --abbc761f78ff4d7cb7573b5a23f96ef0\r\n\
             Content-Disposition: form-data; name=\"files_param\"; filename=\"fn3.txt\"\r\n\
             Content-Type: text/plain; charset=utf-8\r\n\r\n\
             test\r\n\
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

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!(data.file_param.len(), 4),
            Err(_) => panic!("Failed to parse multipart into structure"),
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

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(_) => panic!(
                "Types not matching, but parsing was a success. It should have return an Err(_)"
            ),
            Err(_) => panic!("Failed to parse multipart into structure"),
        }
    }

    #[allow(dead_code)]
    #[actix_rt::test]
    async fn testing_primitive_type_array() {
        #[derive(Deserialize)]
        struct Test {
            string_param: Vec<String>,
            i32_param: Vec<i32>,
            u32_param: Vec<u32>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_array_header();

        sender.send(Ok(bytes)).unwrap();

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!(data.string_param.len(), 3),
            Err(_) => panic!("Failed to parse multipart into structure"),
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

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!(data.file_param.is_none(), true),
            Err(_) => panic!("Failed to parse multipart into structure"),
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

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!(data.file_param.is_some(), true),
            Err(_) => panic!("Failed to parse multipart into structure"),
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

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!(data.file_param.is_none(), true),
            Err(_) => panic!("Failed to parse multipart into structure"),
        }
    }

    #[actix_rt::test]
    async fn test_multiple_files_param_with_3_files() {
        #[derive(Deserialize)]
        struct Test {
            files_param: Vec<File>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_3_files_array_header();

        sender.send(Ok(bytes)).unwrap();

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!((data.files_param.len() == 3), true),
            Err(_) => panic!("Failed to parse multipart into structure"),
        }
    }
    #[actix_rt::test]
    async fn test_multiple_files_param_with_1_file() {
        #[derive(Deserialize)]
        struct Test {
            files_param: Vec<File>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_1_files_array_header();

        sender.send(Ok(bytes)).unwrap();

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!((data.files_param.len() == 1), true),
            Err(_) => panic!("Failed to parse multipart into structure"),
        }
    }

    #[actix_rt::test]
    #[should_panic(
        expected = "When uploading multiple files with one field, the field name need to have hooks [] at the end"
    )]
    async fn test_multiple_files_param_with_3_file_with_name_without_hooks() {
        #[derive(Deserialize)]
        struct Test {
            files_param: Vec<File>,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) =
            create_simple_request_with_3_files_array_with_name_without_hooks_header();

        sender.send(Ok(bytes)).unwrap();

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!((data.files_param.len() == 3), true),
            Err(_) => panic!("When uploading multiple files with one field, the field name need to have hooks [] at the end")
        }
    }

    #[actix_rt::test]
    async fn test_value_in_good_param() {
        #[derive(Deserialize)]
        struct Test {
            param1: u32,
            param2: u32,
        }

        let (sender, payload) = create_stream();
        let (bytes, headers) = create_simple_request_with_header_with_2_u32();

        sender.send(Ok(bytes)).unwrap();

        let actix_multipart = actix_multipart::Multipart::new(&headers, payload);

        match extract_multipart::<Test>(actix_multipart).await {
            Ok(data) => assert_eq!(
                if data.param1 == 56 && data.param2 == 24 {
                    true
                } else {
                    false
                },
                true
            ),
            Err(_) => panic!("Failed to parse multipart into structure"),
        }
    }
}
