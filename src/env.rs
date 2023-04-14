use arcs_deploy_env::*;

env_var_req!(DEPLOY_SERVER_AUTH_TOKEN -> DEPLOY_TOKEN);
env_var_req!(WEBHOOK_SERVER_AUTH_TOKEN -> WEBHOOK_TOKEN);
env_var_req!(WEBHOOK_SERVER_ADDRESS -> WEBHOOK_ADDRESS);
env_var_req!(DEPLOY_SERVER_ADDRESS -> DEPLOY_ADDRESS);
env_var_req!(S3_BUCKET_URL -> S3_ADDRESS);
env_var_req!(S3_DISPLAY_URL -> S3_DISPLAY_ADDRESS);


assert_req_env!(check_env_vars: DEPLOY_TOKEN, WEBHOOK_TOKEN, WEBHOOK_ADDRESS, DEPLOY_ADDRESS, S3_ADDRESS, S3_DISPLAY_ADDRESS);
