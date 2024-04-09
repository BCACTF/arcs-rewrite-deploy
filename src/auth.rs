use std::fmt::{Display, Formatter};

use actix_web::body::BoxBody;
use actix_web::{ResponseError, HttpResponse};
use lazy_static::lazy_static;

use constant_time_eq::constant_time_eq_n;
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web::dev::ServiceRequest;
use actix_web::http::StatusCode as actixStatusCode;
use crate::env::webhook_token;
use crate::logging::*;

#[derive(Debug)]
struct Authentication {
    status_code: actixStatusCode,
    message: &'static str,
}

impl Display for Authentication{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Authentication Error: {{status_code: {}; message: {}}}", self.message, self.status_code)
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

const KEY_SIZE: usize = 64;

lazy_static! {
    // parsed into a [u8;32] for constant time comparison
    static ref WEBHOOKARR : [u8; KEY_SIZE]= match (&webhook_token().as_bytes().to_owned()[..]).try_into() {
        Ok(arr) => arr,
        Err(e) => {
            error!("Error converting from slice to [u8;{KEY_SIZE}]");
            error!("{:?}", e);
            panic!("Failed to convert WEBHOOK_SERVER_AUTH_TOKEN to [u8;{KEY_SIZE}]");
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

    let credarr : [u8; KEY_SIZE]= match (&credentials.token().as_bytes().to_owned()[..]).try_into() {
        Ok(arr) => arr,
        Err(e) => {
            error!("Error converting credentials from slice to [u8;{KEY_SIZE}]");
            warn!("Ensure size is {KEY_SIZE} bytes");
            trace!("{:?}", e);
            return Err((Authentication::BAD_REQUEST.into(), req))
        },
    };

    if constant_time_eq_n(&credarr, &WEBHOOKARR) {
        return Ok(req);
    }

    warn!("Unauthenticated request received");
    Err((Authentication::INVALID_TOKEN.into(), req))
}
