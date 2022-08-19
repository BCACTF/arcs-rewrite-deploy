pub mod database;
pub use std::io::{ Result as IOResult, Error as IOError };

#[allow(unused_macros)]
pub mod logging {
    use arcs_deploy_logging::with_target;
    with_target! { "arcs-deploy" }
}
