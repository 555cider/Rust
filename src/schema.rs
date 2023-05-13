diesel::table! {
    actix.product (id) {
        id -> Int4,
        name -> Varchar,
        stock -> Float8,
        price -> Nullable<Int4>,
    }
}
