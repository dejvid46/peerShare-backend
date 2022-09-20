use actix_web::{
    error,
    http::{header::ContentType, StatusCode},
    HttpResponse,
};
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ResErr {
    InternalError(&'static str),
    BadClientData(&'static str),
    BadClientDataOwned(String),
}

impl Display for ResErr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self {
            &ResErr::InternalError(s) => write!(f, "{}", s),
            &ResErr::BadClientData(s) => write!(f, "{}", s),
            &ResErr::BadClientDataOwned(s) => write!(f, "{}", s),
        }
    }
}

impl error::ResponseError for ResErr {
    fn error_response(&self) -> HttpResponse {
        HttpResponse::build(self.status_code())
            .insert_header(ContentType::html())
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            ResErr::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ResErr::BadClientData(_) => StatusCode::BAD_REQUEST,
            ResErr::BadClientDataOwned(_) => StatusCode::BAD_REQUEST,
        }
    }
}