cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        pub mod error;
        mod user;
        pub use user::{DomainUserInfo, get_user_info};
    } else {
        compile_error!("Crate windows-utils only supports Windows targets");
    }
}
