use diesel::{self, PgConnection, QueryDsl, RunQueryDsl, ExpressionMethods};

use models::{
    posts::Post,
    users::User
};
use schema::post_authors;

#[derive(Queryable, Identifiable, Associations)]
#[belongs_to(Post)]
#[belongs_to(User, foreign_key = "author_id")]
pub struct PostAuthor {
    pub id: i32,
    pub post_id: i32,
    pub author_id: i32
}

#[derive(Insertable)]
#[table_name = "post_authors"]
pub struct NewPostAuthor {
    pub post_id: i32,
    pub author_id: i32
}

impl PostAuthor {
    insert!(post_authors, NewPostAuthor);
    get!(post_authors);
}
