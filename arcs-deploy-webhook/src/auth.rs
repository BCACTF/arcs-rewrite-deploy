use std::fmt::{Display, Formatter};

use actix_web::body::BoxBody;
use actix_web::{ResponseError, HttpResponse};
use lazy_static::lazy_static;

use constant_time_eq::{ constant_time_eq_32 };
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web::dev::ServiceRequest;
use actix_web::http::{StatusCode as actixStatusCode};
use crate::logging::*;

#[derive(Debug)]
struct Authentication {
    status_code: actixStatusCode,
    message: &'static str,
}

impl Display for Authentication{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication Error: {{message: {}; status_code: {}}}", self.message, self.status_code)
    }
}

impl ResponseError for Authentication {
    fn status_code(&self) -> actixStatusCode {
        self.status_code
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        HttpResponse::build(self.status_code)
            .body(self.message)
     }
}

impl Authentication {
    pub const INVALID_TOKEN: Self = Authentication { status_code: actixStatusCode::UNAUTHORIZED, message: "Unauthorized Request" };
    pub const BAD_REQUEST: Self = Authentication { status_code: actixStatusCode::BAD_REQUEST, message: "Malformed Request" };
}

lazy_static! {
    static ref WEBHOOK_SERVER_TOKEN: String = std::env::var("WEBHOOK_SERVER_AUTH_TOKEN").expect("WEBHOOK_SERVER_TOKEN must be set");
    // parsed into a [u8;32] for constant time comparison
    static ref WEBHOOKARR : [u8;32]= match (&WEBHOOK_SERVER_TOKEN.as_bytes().to_owned()[..]).try_into() {
        Ok(arr) => arr,
        Err(e) => {
            error!("Error converting from slice to [u8;32]");
            error!("{:?}", e);
            panic!("Failed to convert WEBHOOK_SERVER_AUTH_TOKEN to [u8;32]");
        },
    };
}

// todo - potentially switch over to jwt? or hmac?
/// Function to validate the authentication token of a request
/// 
/// Reads in from the `Authentication` header of the request
/// 
/// ## Returns
/// - `Ok(ServiceRequest)` - If the token is valid
/// - `Err((actix_web::Error, ServiceRequest))` - If the token is invalid : short circuits request and returns status to client
pub async fn validate_auth_token (
    req: ServiceRequest,
    credentials: BearerAuth,
) -> Result<ServiceRequest, (actix_web::Error, ServiceRequest)> {

    let credarr : [u8;32]= match (&credentials.token().as_bytes().to_owned()[..]).try_into() {
        Ok(arr) => arr,
        Err(e) => {
            error!("Error converting credentials from slice to [u8;32]");
            warn!("Ensure size is 32 bytes");
            trace!("{:?}", e);
            return Err((Authentication::BAD_REQUEST.into(), req))
        },
    };

    if constant_time_eq_32(&credarr, &WEBHOOKARR) {
        return Ok(req);
    }

    warn!("Unauthenticated request received");
    return Err((Authentication::INVALID_TOKEN.into(), req))
}
