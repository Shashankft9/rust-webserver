use crate::database::Database;
use crate::models::Post;

use iron::headers::ContentType;
use iron::{status, AfterMiddleware, Handler, IronResult, Request, Response};
use router::Router;
use std::io::Read;
use std::sync::{Arc, Mutex};
use uuid::Uuid;
use rand::Rng;
use std::{thread, time};

macro_rules! try_handler {
    ($e:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                return Ok(Response::with((
                    status::InternalServerError,
                    e.to_string(),
                )))
            }
        }
    };
    ($e:expr, $error:expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => return Ok(Response::with(($error, e.to_string()))),
        }
    };
}

macro_rules! lock {
    ($e:expr) => {
        $e.lock().unwrap()
    };
}

macro_rules! get_http_param {
    ($r:expr, $e:expr) => {
        match $r.extensions.get::<Router>() {
            Some(router) => match router.find($e) {
                Some(v) => v,
                None => return Ok(Response::with(status::BadRequest)),
            },
            None => return Ok(Response::with(status::InternalServerError)),
        }
    };
}

pub struct Handlers {
    pub post_feed: PostFeedHandler,
    pub post_post: PostPostHandler,
    pub post: PostHandler,
    pub get_helloworld: GetHelloworldHandler,
    pub get_headers: GetHeaders,
    pub resp_503: Resp503,
    pub resp_delay: RespDelay,
}

impl Handlers {
    pub fn new(db: Database) -> Handlers {
        let database = Arc::new(Mutex::new(db));
        Handlers {
            post_feed: PostFeedHandler::new(database.clone()),
            post_post: PostPostHandler::new(database.clone()),
            post: PostHandler::new(database.clone()),
            get_helloworld: GetHelloworldHandler{},
            get_headers: GetHeaders{},
            resp_503: Resp503{},
            resp_delay: RespDelay{},
        }
    }
}

pub struct GetHelloworldHandler {
}

impl Handler for GetHelloworldHandler{
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let payload = String::from("hello from rust webserver");
        Ok(Response::with((status::Ok, payload)))
    }
}

pub struct GetHeaders {
}

impl Handler for GetHeaders{
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let payload = req.headers.to_string();
        Ok(Response::with((status::Ok, payload)))
    }
}

pub struct Resp503 {
}

impl Handler for Resp503{
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let mut rng = rand::thread_rng();
        let tmp = rng.gen_range(0..10);
        if tmp%2 == 0 {
            Ok(Response::with((status::Ok, String::from("try again for 503"))))
        } else {
            Ok(Response::with(status::ServiceUnavailable))
        }
    }
}
pub struct RespDelay {
}

impl Handler for RespDelay{
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref resp_delay = get_http_param!(req, "delay");
        let resp_delay_int: u64 = resp_delay.parse().unwrap();        
        thread::sleep(time::Duration::from_secs(resp_delay_int));
        let payload = format!("hello from rust webserver after {} seconds delay, thanks for being patient", resp_delay_int);
        Ok(Response::with((status::Ok, payload)))
    }
}
pub struct PostFeedHandler {
    database: Arc<Mutex<Database>>,
}

impl PostFeedHandler {
    fn new(database: Arc<Mutex<Database>>) -> PostFeedHandler {
        PostFeedHandler { database }
    }
}

impl Handler for PostFeedHandler {
    fn handle(&self, _: &mut Request) -> IronResult<Response> {
        let payload = try_handler!(serde_json::to_string(lock!(self.database).posts()));
        Ok(Response::with((status::Ok, payload)))
    }
}

pub struct PostPostHandler {
    database: Arc<Mutex<Database>>,
}

impl PostPostHandler {
    fn new(database: Arc<Mutex<Database>>) -> PostPostHandler {
        PostPostHandler { database }
    }
}

impl Handler for PostPostHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let mut payload = String::new();
        try_handler!(req.body.read_to_string(&mut payload));

        let post = try_handler!(serde_json::from_str(payload.as_str()), status::BadRequest);

        lock!(self.database).add_post(post);
        Ok(Response::with((status::Created, payload)))
    }
}

pub struct PostHandler {
    database: Arc<Mutex<Database>>,
}

impl PostHandler {
    fn new(database: Arc<Mutex<Database>>) -> PostHandler {
        PostHandler { database }
    }

    fn find_post(&self, id: &Uuid) -> Option<Post> {
        let locked = lock!(self.database);
        let mut iterator = locked.posts().iter();
        iterator.find(|p| p.uuid() == id).map(|p| p.clone())
    }
}

impl Handler for PostHandler {
    fn handle(&self, req: &mut Request) -> IronResult<Response> {
        let ref post_id = get_http_param!(req, "id");

        let id = try_handler!(Uuid::parse_str(post_id), status::BadRequest);

        if let Some(post) = self.find_post(&id) {
            let payload = try_handler!(serde_json::to_string(&post), status::InternalServerError);
            Ok(Response::with((status::Ok, payload)))
        } else {
            Ok(Response::with(status::NotFound))
        }
    }
}

pub struct JsonAfterMiddleware;

impl AfterMiddleware for JsonAfterMiddleware {
    fn after(&self, _: &mut Request, mut res: Response) -> IronResult<Response> {
        res.headers.set(ContentType::json());
        Ok(res)
    }
}