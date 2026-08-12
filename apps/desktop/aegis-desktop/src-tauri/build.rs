fn main() {
    tauri_build::build();
    let url = std::env::var("AEGIS_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8080".into());
    println!("cargo:rustc-env=AEGIS_SERVER_URL={url}");
}
