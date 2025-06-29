use dotenvy::from_filename;
use env::var;
use std::env;
use std::path::Path;
use std::sync::OnceLock;

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn get_config() -> &'static AppConfig {
    CONFIG.get_or_init(|| AppConfig::from_env())
}

#[derive(Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub database_url: String,
    pub server_host: String,
    pub server_port: u16,
    pub log: Log,
    pub token: Token,
    pub cookie: Cookie,
}

impl AppConfig {
    pub fn from_env() -> Self {
        load_env_files();

        Self {
            app_name: var("APP_NAME").unwrap_or("admin-server".to_string()),
            database_url: var("DATABASE_URL").expect("DATABASE_URL must be set"),
            server_host: var("SERVER_HOST").expect("SERVER_HOST must be set"),
            server_port: var("SERVER_PORT")
                .expect("SERVER_PORT must be a valid number")
                .parse()
                .expect("SERVER_PORT must be a valid number"),
            log: Log::from_env(),
            token: Token::from_env(),
            cookie: Cookie::from_env(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Log {
    pub console_enable: bool,
    pub console_level: String, // "debug", "info", "warn", "error"
    pub file_enable: bool,
    pub file_path: String,
    pub file_level: String, // "debug", "info", "warn", "error"
}

impl Log {
    pub fn from_env() -> Self {
        Self {
            console_enable: var("LOG_CONSOLE_ENABLE")
                .unwrap_or("true".to_string())
                .parse()
                .expect("LOG_CONSOLE_ENABLE must be a valid boolean"),
            console_level: var("RUST_LOG").unwrap_or("info".to_string()),
            file_enable: var("LOG_FILE_ENABLE")
                .unwrap_or("true".to_string())
                .parse()
                .expect("LOG_FILE_ENABLE must be a valid boolean"),
            file_level: var("LOG_FILE_LEVEL").unwrap_or("info".to_string()),
            file_path: var("LOG_FILE_PATH").unwrap_or("logs".to_string()),
        }
    }
}

#[derive(Clone)]
pub struct Token {
    pub secret: String,
    pub access_name: String,
    pub access_exp: i64,
    pub refresh_name: String,
    pub refresh_exp: i64,
}

impl Token {
    pub fn from_env() -> Self {
        Self {
            secret: var("TOKEN_SECRET").expect("TOKEN_SECRET must be set"),
            access_name: var("TOKEN_ACCESS_NAME").unwrap_or("access".to_string()),
            access_exp: var("TOKEN_ACCESS_EXP")
                .unwrap_or("3600".to_string())
                .parse()
                .expect("TOKEN_ACCESS_EXP must be a valid number"),
            refresh_name: var("TOKEN_REFRESH_NAME").unwrap_or("refresh".to_string()),
            refresh_exp: var("TOKEN_REFRESH_EXP")
                .unwrap_or("86400".to_string())
                .parse()
                .expect("TOKEN_REFRESH_EXP must be a valid number"),
        }
    }
}

#[derive(Clone)]
pub struct Cookie {
    pub access_token_name: String,
    pub access_token_max_age: i64,
    pub refresh_token_name: String,
    pub refresh_token_max_age: i64,
    pub domain: String,
    pub secure: bool,
    pub http_only: bool,
    pub same_site: String,
}

impl Cookie {
    pub fn from_env() -> Self {
        Self {
            access_token_name: var("COOKIE_ACCESS_TOKEN_NAME")
                .unwrap_or("access_token".to_string()),
            access_token_max_age: var("COOKIE_ACCESS_TOKEN_MAX_AGE")
                .unwrap_or("3600".to_string())
                .parse()
                .expect("COOKIE_ACCESS_TOKEN_MAX_AGE must be a valid number"),
            refresh_token_name: var("COOKIE_REFRESH_TOKEN_NAME")
                .unwrap_or("refresh_token".to_string()),
            refresh_token_max_age: var("COOKIE_REFRESH_TOKEN_MAX_AGE")
                .unwrap_or("86400".to_string())
                .parse()
                .expect("COOKIE_REFRESH_TOKEN_MAX_AGE must be a valid number"),
            domain: var("COOKIE_DOMAIN").unwrap_or("localhost".to_string()),
            secure: var("COOKIE_SECURE")
                .unwrap_or("false".to_string())
                .parse()
                .expect("COOKIE_SECURE must be a valid boolean"),
            http_only: var("COOKIE_HTTP_ONLY")
                .unwrap_or("false".to_string())
                .parse()
                .expect("COOKIE_HTTP_ONLY must be a valid boolean"),
            same_site: var("COOKIE_SAME_SITE").unwrap_or("lax".to_string()),
        }
    }
}

fn load_env_files() {
    // 환경 확인
    let rust_env = var("RUST_ENV").unwrap_or_else(|_| "dev".to_string());

    let env_files = vec![
        ".env".to_string(),           // 1. 기본값
        format!(".env.{}", rust_env), // 2. 환경별
    ];

    for file in env_files {
        if Path::new(&file).exists() {
            match from_filename(&file) {
                Ok(_) => println!("✅ Loaded `{}`", file),
                Err(e) => println!("❌ Failed to load `{}`: {}", file, e),
            }
        } else {
            println!("⚠️ `{}` not found (skipping)", file);
        }
    }

    println!("🚀 Environment: {}", rust_env);
}
