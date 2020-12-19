use dropbox_sdk::default_client::{NoauthDefaultClient, UserAuthDefaultClient};
use dropbox_sdk::oauth2::{
    oauth2_token_from_authorization_code, Oauth2AuthorizeUrlBuilder, Oauth2Type,
};
use dropbox_sdk::{files, UserAuthClient};
use std::env;

pub fn list_directory(path: &str) {
    let client = UserAuthDefaultClient::new(get_oauth2_token());
    let requested_path = if path == "/" {
        String::new()
    } else {
        path.to_owned()
    };
    match files::list_folder(
        &client,
        &files::ListFolderArg::new(requested_path).with_recursive(false),
    ) {
        Ok(Ok(result)) => {
            println!("{:?}", result)
        }
        Ok(Err(e)) => {
            println!("{}", e)
        }
        Err(e) => {
            println!("{}", e)
        }
    };
}

pub fn get_file_metadata(path: &str) {
    let client = UserAuthDefaultClient::new(get_oauth2_token());
    match files::get_metadata(
        &client,
        &files::GetMetadataArg::new(path.to_string()).with_include_media_info(true),
    ) {
        Ok(Ok(result)) => {
            println!("{:?}", result)
        }
        Ok(Err(e)) => {
            println!("{}", e)
        }
        Err(e) => {
            println!("{}", e)
        }
    };
}

fn get_oauth2_token() -> String {
    env::var("DBX_OAUTH_TOKEN").unwrap()
}
