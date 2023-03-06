extern crate pretty_env_logger;
use chrono::{DateTime, Local, TimeZone};
use chrono::Timelike;
use futures::executor::block_on;
use std::io::BufReader;
use std::{thread, time};
use actix_files as fs;
use actix_web::{Error, error, post, put, get, Responder, App, web, HttpServer, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use futures::StreamExt;
use web_push::*;
use std::fs::File;
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};
use std::fs::OpenOptions;
use std::io::prelude::*;

const MAX_SIZE: usize = 262_144; // max payload size is 256k

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Keys {
    p256dh: String,
    auth: String,
}
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct Subscription {
    endpoint: String,
    keys: Keys,
}

async fn bedtime(subscription_info: SubscriptionInfo) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut vapid_private_key: Option<String> = Some("private_key.pem".to_string());
    let mut push_payload: Option<String> = Some("hi".to_string());
    let mut encoding: Option<String> = Some("aes128gcm".to_string());
    let mut ttl: Option<u32> = Some(1);

    let ece_scheme = match encoding.as_deref() {
        Some("aes128gcm") => ContentEncoding::Aes128Gcm,
        None => ContentEncoding::Aes128Gcm,
        Some(_) => panic!("Content encoding can only be 'aes128gcm'"),
    };

    let mut builder = WebPushMessageBuilder::new(&subscription_info).unwrap();

    if let Some(ref payload) = push_payload {
        builder.set_payload(ece_scheme, payload.as_bytes());
    }

    if let Some(time) = ttl {
        builder.set_ttl(time);
    }

    if let Some(ref vapid_file) = vapid_private_key {
        let file = File::open(vapid_file).unwrap();

        let mut sig_builder = VapidSignatureBuilder::from_pem(file, &subscription_info).unwrap();

        sig_builder.add_claim("sub", "mailto:test@example.com");
        sig_builder.add_claim("foo", "bar");
        sig_builder.add_claim("omg", 123);

        let signature = sig_builder.build().unwrap();

        builder.set_vapid_signature(signature);
        builder.set_payload(ContentEncoding::Aes128Gcm, "test".as_bytes());
    };

    let client = WebPushClient::new()?;

    let response = client.send(builder.build()?).await;
    println!("Sent out a notifications");

    Ok(())

}

#[put("/save-subscription/")]
async fn save_subcription(mut payload: web::Payload) -> Result<HttpResponse, Error> {
    // payload is a stream of Bytes objects
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    // body is loaded, now we can deserialize serde-json
    let obj = serde_json::from_slice::<SubscriptionInfo>(&body)?;
    
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open("subscribers")
        .unwrap();

    if let Err(e) = writeln!(file, "{}", serde_json::to_string(&obj).unwrap()) {
        eprintln!("could not write to file");
    }


    Ok(HttpResponse::Ok().body("hi"))
}

#[get("/")]
async fn index(req: HttpRequest) -> impl Responder {
    println!("index");
    let response = std::fs::read_to_string("./static/index.html").unwrap();


    HttpResponse::Ok()
        .content_type(ContentType::html())
        .insert_header(("Service-Worker-Allowed", "/"))
        .body(response)
}


#[actix_web::main]
async fn main() -> std::io::Result<()> {
    pretty_env_logger::init();

    thread::spawn(move || {
        loop {
            let dt = time::Duration::from_millis(500);
            thread::sleep(dt);

            let berlin: chrono::FixedOffset = chrono::FixedOffset::east(1 * 3600); // Berlin timezone is UTC+1
            let now_berlin: DateTime<chrono::FixedOffset> = Local::now().with_timezone(&berlin);

            let h = now_berlin.hour();
            let m = now_berlin.minute();
            dbg!(h,m);
            if !(h == 22 && m == 30) {
                continue;
            }

            let file = if let Ok(line) = File::open("subscribers") {
                line
            } else {
                eprintln!("could not open subscribers");   
                continue
            };
            let lines = BufReader::new(file).lines();
            for line in lines {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => continue,
                };
                let obj = serde_json::from_str(&line).unwrap();
                match block_on(bedtime(obj)) {
                    Err(e) => println!("{e}"),
                    _ => {},
                };

            }
            let dt = time::Duration::from_secs(65);
            thread::sleep(dt);

        }
    });

    HttpServer::new(|| App::new()
                    .service(index)
                    .service(save_subcription)
                    .service(fs::Files::new("/", "./static").show_files_listing()))
        .bind(("127.0.0.1", 4000))?
        .run()
        .await
} 
