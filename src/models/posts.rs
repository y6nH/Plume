use activitypub::{
    activity::Create,
    link,
    object::{Article, properties::ObjectProperties}
};
use chrono::NaiveDateTime;
use diesel::{self, PgConnection, RunQueryDsl, QueryDsl, ExpressionMethods, BelongingToDsl, dsl::any};
use serde_json;

use BASE_URL;
use activity_pub::{
    PUBLIC_VISIBILTY, ap_url, Id, IntoId,
    inbox::FromActivity
};
use models::{
    blogs::Blog,
    instance::Instance,
    likes::Like,
    mentions::Mention,
    post_authors::PostAuthor,
    reshares::Reshare,
    users::User
};
use schema::posts;
use safe_string::SafeString;

#[derive(Queryable, Identifiable, Serialize, Clone)]
pub struct Post {
    pub id: i32,
    pub blog_id: i32,
    pub slug: String,
    pub title: String,
    pub content: SafeString,
    pub published: bool,
    pub license: String,
    pub creation_date: NaiveDateTime,
    pub ap_url: String
}

#[derive(Insertable)]
#[table_name = "posts"]
pub struct NewPost {
    pub blog_id: i32,    
    pub slug: String,
    pub title: String,
    pub content: SafeString,
    pub published: bool,
    pub license: String,
    pub ap_url: String
}

impl Post {
    insert!(posts, NewPost);
    get!(posts);
    find_by!(posts, find_by_slug, slug as String, blog_id as i32);
    find_by!(posts, find_by_ap_url, ap_url as String);

    pub fn count_local(conn: &PgConnection) -> usize {
        use schema::post_authors;
        use schema::users;
        let local_authors = users::table.filter(users::instance_id.eq(Instance::local_id(conn))).select(users::id);
        let local_posts_id = post_authors::table.filter(post_authors::author_id.eq(any(local_authors))).select(post_authors::post_id);
        posts::table.filter(posts::id.eq(any(local_posts_id)))
            .load::<Post>(conn)
            .expect("Couldn't load local posts")
            .len()
    }

    pub fn get_recents(conn: &PgConnection, limit: i64) -> Vec<Post> {
        posts::table.order(posts::creation_date.desc())
            .limit(limit)
            .load::<Post>(conn)
            .expect("Error loading recent posts")
    }

    pub fn get_recents_for_author(conn: &PgConnection, author: &User, limit: i64) -> Vec<Post> {
        use schema::post_authors;

        let posts = PostAuthor::belonging_to(author).select(post_authors::post_id);
        posts::table.filter(posts::id.eq(any(posts)))
            .order(posts::creation_date.desc())
            .limit(limit)
            .load::<Post>(conn)
            .expect("Error loading recent posts for author")
    }

    pub fn get_recents_for_blog(conn: &PgConnection, blog: &Blog, limit: i64) -> Vec<Post> {
        posts::table.filter(posts::blog_id.eq(blog.id))
            .order(posts::creation_date.desc())
            .limit(limit)
            .load::<Post>(conn)
            .expect("Error loading recent posts for blog")
    }

    pub fn get_authors(&self, conn: &PgConnection) -> Vec<User> {
        use schema::users;
        use schema::post_authors;
        let author_list = PostAuthor::belonging_to(self).select(post_authors::author_id);
        users::table.filter(users::id.eq(any(author_list))).load::<User>(conn).unwrap()
    }

    pub fn get_blog(&self, conn: &PgConnection) -> Blog {
        use schema::blogs;
        blogs::table.filter(blogs::id.eq(self.blog_id))
            .limit(1)
            .load::<Blog>(conn)
            .expect("Couldn't load blog associted to post")
            .into_iter().nth(0).unwrap()
    }

    pub fn get_likes(&self, conn: &PgConnection) -> Vec<Like> {
        use schema::likes;
        likes::table.filter(likes::post_id.eq(self.id))
            .load::<Like>(conn)
            .expect("Couldn't load likes associted to post")
    }

    pub fn get_reshares(&self, conn: &PgConnection) -> Vec<Reshare> {
        use schema::reshares;
        reshares::table.filter(reshares::post_id.eq(self.id))
            .load::<Reshare>(conn)
            .expect("Couldn't load reshares associted to post")
    }

    pub fn update_ap_url(&self, conn: &PgConnection) {
        if self.ap_url.len() == 0 {
            diesel::update(self)
                .set(posts::ap_url.eq(self.compute_id(conn)))
                .get_result::<Post>(conn).expect("Couldn't update AP URL");
        }
    }

    pub fn get_receivers_urls(&self, conn: &PgConnection) -> Vec<String> {
        let followers = self.get_authors(conn).into_iter().map(|a| a.get_followers(conn)).collect::<Vec<Vec<User>>>();
        let to = followers.into_iter().fold(vec![], |mut acc, f| {
            for x in f {
                acc.push(x.ap_url);
            }
            acc
        });
        to
    }

    pub fn into_activity(&self, conn: &PgConnection) -> Article {
        let mut to = self.get_receivers_urls(conn);
        to.push(PUBLIC_VISIBILTY.to_string());

        let mentions = Mention::list_for_post(conn, self.id).into_iter().map(|m| m.to_activity(conn)).collect::<Vec<link::Mention>>();

        let mut article = Article::default();
        article.object_props = ObjectProperties {
            name: Some(serde_json::to_value(self.title.clone()).unwrap()),
            id: Some(serde_json::to_value(self.ap_url.clone()).unwrap()),
            attributed_to: Some(serde_json::to_value(self.get_authors(conn).into_iter().map(|x| x.ap_url).collect::<Vec<String>>()).unwrap()),
            content: Some(serde_json::to_value(self.content.clone()).unwrap()),
            published: Some(serde_json::to_value(self.creation_date).unwrap()),
            tag: Some(serde_json::to_value(mentions).unwrap()),
            url: Some(serde_json::to_value(self.ap_url.clone()).unwrap()),
            to: Some(serde_json::to_value(to).unwrap()),
            cc: Some(serde_json::to_value(Vec::<serde_json::Value>::new()).unwrap()),
            ..ObjectProperties::default()
        };
        article
    }

    pub fn create_activity(&self, conn: &PgConnection) -> Create {
        let mut act = Create::default();
        act.object_props.set_id_string(format!("{}/activity", self.ap_url)).unwrap();
        act.create_props.set_actor_link(Id::new(self.get_authors(conn)[0].clone().ap_url)).unwrap();
        act.create_props.set_object_object(self.into_activity(conn)).unwrap();
        act
    }

    pub fn to_json(&self, conn: &PgConnection) -> serde_json::Value {
        json!({
            "post": self,
            "author": self.get_authors(conn)[0].to_json(conn),
            "url": format!("/~/{}/{}/", self.get_blog(conn).actor_id, self.slug),
            "date": self.creation_date.timestamp()
        })
    }

    pub fn compute_id(&self, conn: &PgConnection) -> String {
        ap_url(format!("{}/~/{}/{}/", BASE_URL.as_str(), self.get_blog(conn).actor_id, self.slug))
    }
}

impl FromActivity<Article> for Post {
    fn from_activity(conn: &PgConnection, article: Article, _actor: Id) -> Post {
        let post = Post::insert(conn, NewPost {
            blog_id: 0, // TODO
            slug: String::from(""), // TODO
            title: article.object_props.name_string().unwrap(),
            content: SafeString::new(&article.object_props.content_string().unwrap()),
            published: true,
            license: String::from("CC-0"),
            ap_url: article.object_props.url_string().unwrap_or(String::from(""))
        });

        // save mentions
        if let Some(serde_json::Value::Array(tags)) = article.object_props.tag.clone() {
            for tag in tags.into_iter() {
                serde_json::from_value::<link::Mention>(tag)
                    .map(|m| Mention::from_activity(conn, m, post.id, true))
                    .ok();
            }
        }
        post
    }
}

impl IntoId for Post {
    fn into_id(self) -> Id {
        Id::new(self.ap_url.clone())
    }
}
