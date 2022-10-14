pub mod database;
pub mod webhooks;

pub use std::io::{ Result as IOResult, Error as IOError };

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}


pub fn verify_env() -> Result<(), String> {
    dotenv::dotenv().map_err(|_| "dotenv failed".to_string())?;

    // arcs_deploy_docker::verify_env()
    unimplemented!();
}

pub mod actix_prelude {
    pub use actix_web::{
        HttpRequest,
        HttpResponse,
        Responder as ActixResponder,
        dev::RequestHead,
    };

    pub use actix_web::{
        head,
        get,
        post,
        patch,
        put,
        delete,
    };

    use actix_web::body::BoxBody;
    pub fn accepted(body: &'static str) -> HttpResponse<BoxBody> {
        HttpResponse::Accepted().body(body)
    }
    pub fn bad_request(body: &'static str) -> HttpResponse<BoxBody> {
        HttpResponse::BadRequest().body(body)
    }
    pub fn unauthorized(body: &'static str) -> HttpResponse<BoxBody> {
        HttpResponse::Unauthorized().body(body)
    }

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __response_bail {
        ($macro_name:ident $body:literal) => {
            return $macro_name($body)
        };
    }
    pub use __response_bail as response_bail;

    #[doc(hidden)]
    #[macro_export]
    macro_rules! __bail_on_err {
        ($result:expr) => {
            match $result {
                Ok(num) => num,
                Err(response) => return response,
            }
        };
        ($result:expr, $if_error:expr) => {
            match $result {
                Ok(num) => num,
                Err(_) => return $if_error,
            }
        };
    }
    pub use __bail_on_err as bail_on_err;
}

