#[cfg(target_os="macos")]
pub mod dylib;

#[cfg(target_os="windows")]
pub mod dotnet;
