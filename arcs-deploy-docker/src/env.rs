use arcs_deploy_env::*;

env_var_req!(DOCKER_REGISTRY_USERNAME -> REG_USERNAME);
env_var_req!(DOCKER_REGISTRY_PASSWORD -> REG_PASSWORD);
env_var_req!(DOCKER_REGISTRY_URL -> REG_URL);
env_var_req!(CHALL_FOLDER -> CHALL_FOLDER_DEFAULT);


assert_req_env!(check_env_vars: REG_USERNAME, REG_PASSWORD, REG_URL, CHALL_FOLDER_DEFAULT);