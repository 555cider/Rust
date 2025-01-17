use crate::schema::product;
use diesel::{PgConnection, QueryDsl, RunQueryDsl};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct ProductList(pub Vec<Product>);

#[derive(Queryable, Serialize, Deserialize)]
#[diesel(table_name = product)]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub stock: f64,
    pub price: Option<i32>,
}

#[derive(Insertable, Deserialize, AsChangeset)]
#[diesel(table_name = product)]
pub struct NewProduct {
    pub name: Option<String>,
    pub stock: Option<f64>,
    pub price: Option<i32>,
}

impl ProductList {
    pub fn list(conn: &mut PgConnection) -> Result<Self, diesel::result::Error> {
        use crate::schema::product::dsl::*;

        let result = product
            .limit(10)
            .load::<Product>(conn)
            .expect("Error loading product");

        Ok(ProductList(result))
    }
}

impl NewProduct {
    pub fn create(&self, conn: &mut PgConnection) -> Result<Product, diesel::result::Error> {
        use crate::schema::product::dsl::*;
        diesel::insert_into(product).values(self).get_result(conn)
    }
}

impl Product {
    pub fn find(id: &i32, conn: &mut PgConnection) -> Result<Product, diesel::result::Error> {
        product::table.find(id).first(conn)
    }

    pub fn delete(id: &i32, conn: &mut PgConnection) -> Result<(), diesel::result::Error> {
        diesel::delete(product::dsl::product.find(id)).execute(conn)?;
        Ok(())
    }

    pub fn update(
        id: &i32,
        new_product: &NewProduct,
        conn: &mut PgConnection,
    ) -> Result<(), diesel::result::Error> {
        diesel::update(product::dsl::product.find(id))
            .set(new_product)
            .execute(conn)?;
        Ok(())
    }
}
