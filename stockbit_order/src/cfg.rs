use once_cell::sync::Lazy;

#[derive(serde::Deserialize)]
pub struct AppConfig {
    pub jwt_public_key: String,
    pub database_url: String,
    pub redis_url: String,
}

// Initialize config once
pub static CONFIG: Lazy<AppConfig> = Lazy::new(|| {
    dotenvy::dotenv().ok();

    config::Config::builder()
        .add_source(config::Environment::default())
        .build()
        .expect("")
        .try_deserialize()
        .expect("env not ready")
});
