use include_dir::{Dir, include_dir};

pub const SHARED_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/theme/shared");
pub const THEME_DEFAULT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/theme/default");
pub const THEME_API: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/theme/api");
pub const THEME_BOOK: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/src/theme/book");

pub fn get_embedded(name: &str) -> Option<&'static Dir<'static>> {
    match name {
        "default" => Some(&THEME_DEFAULT),
        "api" => Some(&THEME_API),
        "book" => Some(&THEME_BOOK),
        _ => None,
    }
}

pub fn shared_assets() -> &'static Dir<'static> {
    &SHARED_ASSETS
}
