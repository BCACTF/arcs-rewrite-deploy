use crate::actix_prelude::*;
use crate::logging::*;

mod deserialize {
    use serde::{Deserialize, Deserializer};
    use serde::de::{Error, Unexpected};

    fn deserialize_branch<'de, D>(deserializer: D) -> Result<String, D::Error> where D: Deserializer<'de> {
        let string = String::deserialize(deserializer)?;

        match string.strip_prefix("refs/heads/") {
            Some(branch) => Ok(branch.to_string()),
            None => Err(
                Error::invalid_value(
                    Unexpected::Str(&string),
                    &"refs/heads/<branch-name>",
                ),
            ),
        }
    }


    #[derive(Debug, Deserialize)]
    pub struct GHMainPayload {
        pub repository: GHRepoPayload,
        #[serde(deserialize_with = "deserialize_branch", rename = "ref")]
        pub branch: String,
    }

    impl GHMainPayload {
        pub fn branch_matches(&self) -> bool {
            self.branch == self.repository.default_branch
        }
    }

    #[derive(Debug, Deserialize)]
    pub struct GHRepoPayload {
        pub default_branch: String,

    }
}

pub use deserialize::*;    


use lazy_static::lazy_static;
use std::sync::{RwLock, Arc};
lazy_static!{
    static ref WEBHOOK_SECRET: RwLock<Option<Arc<String>>> = RwLock::new(
        match std::env::var("GH_WEBHOOK_SECRET") {
            Ok(val) => Some(Arc::new(val)),
            Err(_) => None,
        }
    );
}

#[derive(Debug)]
pub enum AuthError {
    NoConfSecret,
    NoSigInHead,
    InvalSig,

    HeadFormatErr,

    LockErr,
    VerifyProcErr,
}

type Hmac256 = hmac::Hmac<sha2::Sha256>;

pub fn verify_auth(req_head: &RequestHead, req_body: &[u8]) -> Result<(), AuthError> {
    use AuthError::*;

    let guard = WEBHOOK_SECRET.read().map_err(|_| LockErr)?;
    let secret = &**guard.as_ref().ok_or(NoConfSecret)?;
    let hash_header = req_head.headers().get("X-Hub-Signature-256").ok_or(NoSigInHead)?;
    let hash_digest = hash_header.as_bytes().strip_prefix("sha256=".as_bytes()).ok_or(HeadFormatErr)?;

    let hash = hex::decode(hash_digest).map_err(|_| HeadFormatErr)?;


    use hmac::Mac;
    let mac = Hmac256::new_from_slice(secret.as_bytes()).map_err(|_| VerifyProcErr)?;

    let mac = mac.chain_update(req_body);

    match mac.verify_slice(&hash) {
        Ok(_) => Ok(()),
        Err(_) => Err(InvalSig)
    }
}

fn branch_matches(payload: &GHMainPayload) -> Result<(), HttpResponse> {
    if payload.branch_matches() {
        Ok(())
    } else {
        warn!("Default and updated branches do not match");
        Err(bad_request(r#"Invalid: ["branch"]"#))
    }
}

#[post("/github_webhook")]
async fn github_webhook(req: HttpRequest, req_body: String) -> impl ActixResponder {

    trace!("Github webhook requested");

    let req_head = req.head();

    if let Some(Ok("push")) = req_head
            .headers()
            .get("X-GitHub-Event")
            .map(|header| header.to_str()) {
        trace!("Confirmed push event.");
    } else {
        response_bail!(bad_request r#"Invalid: ["event"]"#);
    }

    let main_payload: GHMainPayload = bail_on_err!(
        serde_json::from_str(&req_body),
        bad_request(r#"Invalid: ["payload"]"#)
    );
    trace!("Deserialized payload");

    bail_on_err!(branch_matches(&main_payload));
    trace!("Branch matches");


    trace!("Valid payload recieved");
    debug!("Parsed payload: {:#?}", main_payload);

    if let Err(e) = verify_auth(req_head, req_body.as_bytes()) {
        warn!("Secret failed to verify");
        debug!("Verify error: {:?}", e);
        response_bail!(unauthorized r#"Invalid: ["authorization"]"#);
    }

    // TODO: Actually update the repo.
    info!("GH Webhook data and auth verification success, updating repo");

    accepted("Payload accepted!")
}

