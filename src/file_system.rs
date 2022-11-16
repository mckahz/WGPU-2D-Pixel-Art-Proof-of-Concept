use image::*;

#[derive(Debug)]
pub enum LoadError {
    PathNotFound(String),
    CantLoadTexture(String),
    WrongTileLayerType(String),
}

pub fn to_asset_path(path: &str) -> String {
    String::from("./assets/") + path
}

pub fn load_image(path: &str) -> Result<DynamicImage, LoadError> {
    let pwd = to_asset_path(path);
    let err_dir = pwd.to_owned();
    let path_object = match std::fs::read(pwd) {
        Ok(p) => p,
        Err(_) => return Err(LoadError::PathNotFound(err_dir)),
    };

    match image::load_from_memory(&path_object) {
        Ok(i) => Ok(i),
        Err(_) => return Err(LoadError::CantLoadTexture(err_dir)),
    }
}
