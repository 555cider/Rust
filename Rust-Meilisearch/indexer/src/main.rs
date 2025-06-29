use bigdecimal::ToPrimitive;
use meilisearch_sdk::{
    client::*,
    settings::{FacetingSettings, Settings},
};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct GeoPoint {
    lat: f64,
    lng: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct City {
    id: String, // geonameid를 id로 사용
    name: String,
    asciiname: String,
    alternatenames: Vec<String>,
    _geo: GeoPoint,
    country_code: String,
    country: String,
    population: i64,
    timezone: String,
}

async fn sync_to_meilisearch(pool: &Pool<Postgres>, client: &Client) -> Result<(), Box<dyn Error>> {
    let index = client.index("cities");

    // PostgreSQL에서 도시 데이터 조회
    let cities = sqlx::query!(
        r#"
        SELECT 
            c.geonameid,
            c.name,
            c.asciiname,
            c.latitude,
            c.longitude,
            c.country_code,
            c.population,
            c.timezone,
            co.name as country_name,
            COALESCE(array_agg(DISTINCT can.alternate_name) FILTER (WHERE can.alternate_name IS NOT NULL), ARRAY[]::text[]) as alternatenames
        FROM cities c
        JOIN countries co ON c.country_code = co.code
        LEFT JOIN city_alternate_names can ON c.geonameid = can.city_geonameid
        GROUP BY 
            c.geonameid, c.name, c.asciiname, c.latitude, c.longitude,
            c.country_code, c.population, c.timezone, co.name
        "#
    )
    .fetch_all(pool)
    .await?;

    // Meilisearch용 데이터 구조로 변환
    let meili_cities: Vec<City> = cities
        .into_iter()
        .map(|row| {
            let alternatenames = row
                .alternatenames
                .unwrap_or_default()
                .into_iter()
                .filter(|name| !name.is_empty())
                .collect();

            City {
                id: row.geonameid,
                name: row.name,
                asciiname: row.asciiname,
                alternatenames,
                _geo: GeoPoint {
                    lat: row.latitude.to_f64().unwrap_or_default(),
                    lng: row.longitude.to_f64().unwrap_or_default(),
                },
                country_code: row.country_code,
                country: row.country_name,
                population: row.population,
                timezone: row.timezone,
            }
        })
        .collect();

    println!("Syncing {} cities to Meilisearch...", meili_cities.len());

    // 문서 추가
    let task = index.add_documents(&meili_cities, Some("id")).await?;

    // 인덱스 설정
    let settings = Settings::new()
        .with_searchable_attributes([
            "name".to_string(),
            "asciiname".to_string(),
            "alternatenames".to_string(),
            "country".to_string(),
        ])
        .with_filterable_attributes([
            "country_code".to_string(),
            "population".to_string(),
            "timezone".to_string(),
            "_geo".to_string(),
        ])
        .with_sortable_attributes(["population".to_string(), "_geo".to_string()])
        .with_faceting(&FacetingSettings::default());

    // 설정 적용
    let settings_update = index.set_settings(&settings).await?;

    // 모든 작업이 완료될 때까지 기다림
    task.wait_for_completion(client, None, None).await?;
    settings_update
        .wait_for_completion(client, None, None)
        .await?;

    println!("Sync completed successfully!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // PostgreSQL 연결
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://my_user:my_password@localhost/my_db")
        .await?;

    // Meilisearch 클라이언트 생성
    let client = Client::new("http://localhost:7700", Some("eSampleMasterKey"))?;

    sync_to_meilisearch(&pool, &client).await?;

    Ok(())
}
