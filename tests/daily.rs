use snek::cache;
use snek::core;

use std::collections::HashSet;
use std::io::Read as _;
use std::path::PathBuf;
use std::thread;

use bytes::buf::BufExt as _;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Method, Request, Response, Server, StatusCode};
use tokio::runtime::Builder;

async fn echo(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    match (req.method(), req.uri().path()) {
        (&Method::POST, "/uploadContent") => {
            println!("{:?}", req);
            let body = req.into_body();
            let body = hyper::body::aggregate(body).await?;
            let mut buf = [0; 262];
            let mut reader = body.reader();
            reader.read_exact(&mut buf[..]).unwrap();

            let as_utf8 = std::str::from_utf8(&buf).unwrap();
            let filename = &as_utf8[247..262];
            let link = format!(
                "{{\"id\":\"xXXx-{}\",\"permalink\":\"https://tes.tdaily/xXXx-{}_pray\"}}",
                filename, filename
            );
            Ok(Response::new(Body::from(link)))
        }

        (&Method::POST, "/webhook") => {
            /*
            use snek::message::WebhookMessage;
            let body = hyper::body::aggregate(req.into_body()).await?;
            let msg: WebhookMessage = serde_json::from_reader(body.reader()).unwrap();
            */
            Ok(Response::new(Body::from("")))
        }

        _ => {
            println!("not found");
            let mut not_found = Response::default();
            *not_found.status_mut() = StatusCode::NOT_FOUND;
            Ok(not_found)
        }
    }
}

async fn spawn_env() {
    let addr = ([127, 0, 0, 1], 8000).into();
    let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(echo)) });
    let server = Server::bind(&addr).serve(service);
    server.await.unwrap();
}

fn dummy_env() {
    thread::spawn(move || {
        let mut rt = Builder::new()
            .basic_scheduler()
            .enable_all()
            .core_threads(1)
            .build()
            .unwrap();
        rt.block_on(spawn_env())
    });
}

fn test_log_dir() -> PathBuf {
    "tests/example_logs".into()
}

fn link_id(link: &str) -> &str {
    &link[24..39]
}

#[tokio::test]
async fn links() {
    dummy_env();

    let log_dir = test_log_dir();
    let mut output = Vec::new();
    let url = "http://127.0.0.1:8000/";

    let cache = cache::Nop {};
    core::links(log_dir, url, &mut output, cache).await.unwrap();

    let mut buffer = String::new();
    output.as_slice().read_to_string(&mut buffer).unwrap();
    println!("{:?}", buffer);

    let expected_ids = [
        "20200407-174541",
        "20200408-233716",
        "20200407-133033",
        "20200330-081749",
    ]
    .iter()
    .collect::<HashSet<_>>();

    let mut lines = buffer.lines().skip(1);
    for _ in 0..expected_ids.len() {
        let line = lines.next().unwrap();
        let id = link_id(line);
        assert!(expected_ids.contains(&id));
    }
}

#[tokio::test]
async fn sender() {
    use snek::core::LogInfo;
    use snek::core::UploadedLog;
    use snek::log::Log;
    use snek::message::Generator;
    use snek::message::WebhookGenerator;
    use snek::parse;
    use snek::sender::Sender;
    use snek::sender::Webhook;
    use snek::upload;

    dummy_env();

    let log_path = "tests/example_logs/Arkk/Codpiece/20200408-233716.zevtc";
    let upload_url = "http://127.0.0.1:8000/";
    let url = "http://127.0.0.1:8000/webhook";

    let message_generator = WebhookGenerator {};
    let mut webhook_message_sender = Webhook::new(url);

    let log = Log::from_file_checked(&log_path).unwrap();
    let (mut encounters, _) = parse::parse(&log).unwrap();
    let encounter = encounters.remove(0);
    let res = upload::push(upload_url, log.path()).await.unwrap();
    let uploaded_log = UploadedLog::new(log, res.permalink);
    let log_info = LogInfo::new(&uploaded_log, encounter);

    let message = message_generator.generate(&[log_info]);

    webhook_message_sender.send(&message).await.unwrap();
}
