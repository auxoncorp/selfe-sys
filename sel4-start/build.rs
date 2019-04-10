extern crate confignoble;
use confignoble::build_helpers::*;

fn main() {
    BuildEnv::request_reruns();
    let config = load_config_from_env_or_default();
    config.print_boolean_feature_flags();
}
