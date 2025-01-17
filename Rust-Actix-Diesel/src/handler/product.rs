use crate::db_connection::{PgPool, PgPooledConnection};
use crate::model::product::NewProduct;
use crate::model::product::Product;
use crate::model::product::ProductList;
use actix_web::{web, Error, HttpRequest, HttpResponse};
use diesel::r2d2::{ConnectionManager, PooledConnection};

fn pg_pool_handler(pool: web::Data<PgPool>) -> Result<PgPooledConnection, HttpResponse> {
    pool.get()
        .map_err(|e| HttpResponse::InternalServerError().json(e.to_string()))
}

pub async fn list_product(
    _req: HttpRequest,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let mut conn: PooledConnection<ConnectionManager<diesel::PgConnection>> =
        pg_pool_handler(pool).unwrap();
    let product_list: ProductList = ProductList::list(&mut conn).unwrap();
    Ok(HttpResponse::Ok().json(product_list))
}

pub async fn find_product(
    id: web::Path<i32>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let mut conn: PooledConnection<ConnectionManager<diesel::PgConnection>> =
        pg_pool_handler(pool).unwrap();
    let product: Product = Product::find(&id, &mut conn).unwrap();
    Ok(HttpResponse::Ok().json(product))
}

pub async fn create_product(
    new_product: web::Json<NewProduct>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let mut conn: PooledConnection<ConnectionManager<diesel::PgConnection>> =
        pg_pool_handler(pool).unwrap();
    let product: Product = web::block(move || new_product.create(&mut conn).unwrap())
        .await
        .unwrap();
    Ok(HttpResponse::Ok().json(product))
}

pub async fn update_product(
    id: web::Path<i32>,
    new_product: web::Json<NewProduct>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let mut conn: PooledConnection<ConnectionManager<diesel::PgConnection>> =
        pg_pool_handler(pool).unwrap();
    Product::update(&id, &new_product, &mut conn).unwrap();
    Ok(HttpResponse::Ok().finish())
}

pub async fn delete_product(
    id: web::Path<i32>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, Error> {
    let mut conn: PooledConnection<ConnectionManager<diesel::PgConnection>> =
        pg_pool_handler(pool).unwrap();
    Product::delete(&id, &mut conn).unwrap();
    Ok(HttpResponse::Ok().finish())
}
