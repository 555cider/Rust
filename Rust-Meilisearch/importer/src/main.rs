use bigdecimal::BigDecimal;
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::{collections::HashSet, error::Error, fs::File, io::BufReader, str::FromStr};

#[derive(Debug, Serialize, Deserialize)]
struct GeoLocation {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct City {
    geonameid: String,
    name: String,
    asciiname: String,
    alternatenames: String,
    #[serde(rename = "_geo")]
    geo: GeoLocation,
    #[serde(rename = "country code")]
    country_code: String,
    population: i64,
    timezone: String,
    country: String,
}

async fn import_data(pool: &Pool<Postgres>, cities: Vec<City>) -> Result<(), Box<dyn Error>> {
    // 고유한 국가 추출
    let mut countries: HashSet<(String, String)> = HashSet::new();

    for city in &cities {
        countries.insert((city.country_code.clone(), city.country.clone()));
    }

    // 국가 데이터 삽입
    for (code, name) in countries {
        sqlx::query!(
            r#"
            INSERT INTO countries (code, name)
            VALUES ($1, $2)
            ON CONFLICT (code) DO NOTHING
            "#,
            code,
            name
        )
        .execute(pool)
        .await?;
    }

    // 도시 데이터 삽입
    for city in &cities {
        // f64를 BigDecimal로 변환
        let latitude = BigDecimal::from_str(&city.geo.lat.to_string())?;
        let longitude = BigDecimal::from_str(&city.geo.lng.to_string())?;

        sqlx::query!(
            r#"
            INSERT INTO cities 
                (geonameid, name, asciiname, latitude, longitude, 
                 country_code, population, timezone)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (geonameid) DO UPDATE SET
                name = EXCLUDED.name,
                asciiname = EXCLUDED.asciiname,
                latitude = EXCLUDED.latitude,
                longitude = EXCLUDED.longitude,
                country_code = EXCLUDED.country_code,
                population = EXCLUDED.population,
                timezone = EXCLUDED.timezone,
                updated_at = CURRENT_TIMESTAMP
            "#,
            city.geonameid,
            city.name,
            city.asciiname,
            latitude,
            longitude,
            city.country_code,
            city.population,
            city.timezone
        )
        .execute(pool)
        .await?;

        // 대체 이름 처리
        if !city.alternatenames.is_empty() {
            for alt_name in city.alternatenames.split(',') {
                if !alt_name.trim().is_empty() {
                    sqlx::query!(
                        r#"
                        INSERT INTO city_alternate_names (city_geonameid, alternate_name)
                        VALUES ($1, $2)
                        ON CONFLICT DO NOTHING
                        "#,
                        city.geonameid,
                        alt_name.trim()
                    )
                    .execute(pool)
                    .await?;
                }
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // 데이터베이스 연결 풀 생성
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://my_user:my_password@localhost/my_db")
        .await?;

    // JSON 파일 읽기
    println!("Reading JSON file...");
    let file = File::open("world-cities.json")?;
    let reader = BufReader::new(file);
    let cities: Vec<City> = serde_json::from_reader(reader)?;

    println!("Starting import of {} cities...", cities.len());
    import_data(&pool, cities).await?;
    println!("Import completed successfully!");

    Ok(())
}
