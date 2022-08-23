pub mod database;
pub use std::io::{ Result as IOResult, Error as IOError };

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }

}


// TODO : this function
// pub fn verify_env() -> Result<(), String> {
//     dotenv::dotenv().map_err(|_| "dotenv failed".clone())?;

//     // arcs_deploy_docker::verify_env()
//     unimplemented!();
// }
