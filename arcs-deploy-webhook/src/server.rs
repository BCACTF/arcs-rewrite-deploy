pub mod emitter;
pub mod receiver;

use warp::Filter;

pub async fn initialize_server() {
    // let route = warp::any()
    //     .map(|| "Hello, World!");
    
    let route = warp::post()
        .map(|| {
            println!("redeploying challenge");
            format!("redeploying challenge")
        });

    warp::serve(route)
        .run(([127, 0, 0, 1], 3000))
        .await;
}