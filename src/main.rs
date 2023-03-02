use actix_files as fs;
use actix_web::{Error, error, post, put, get, Responder, App, web, HttpServer, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use futures::StreamExt;
use web_push::*;
use std::fs::File;
use base64::{Engine as _, engine::{self, general_purpose}, alphabet};

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

async fn bedtime(s: Subscription) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    const engine: engine::GeneralPurpose =
        engine::GeneralPurpose::new(&alphabet::URL_SAFE, general_purpose::NO_PAD);

    let subscription_info = SubscriptionInfo::new(
        s.endpoint,
        engine.encode(s.keys.p256dh),
        engine.encode(s.keys.auth)
    );
    dbg!(&subscription_info);

    //Read signing material for payload.
    let file = File::open("private_key.pem").unwrap();
    let mut sig_builder = VapidSignatureBuilder::from_pem(file, &subscription_info)?.build()?;

    //Now add payload and encrypt.
    let mut builder = WebPushMessageBuilder::new(&subscription_info)?;
    let content = "Wow!".as_bytes();
    builder.set_payload(ContentEncoding::Aes128Gcm, content);
    builder.set_vapid_signature(sig_builder);

    let client = WebPushClient::new()?;

    //Finally, send the notification!
    client.send(builder.build()?).await?;
    println!("send out");
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
    let obj = serde_json::from_slice::<Subscription>(&body)?;
    
    match bedtime(obj).await {
        Err(e) => println!("{e}"),
        _ => {},
    }

    Ok(HttpResponse::Ok().body("hi")) // <- send response
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
    HttpServer::new(|| App::new()
                    .service(index)
                    .service(save_subcription)
                    .service(fs::Files::new("/", "./static").show_files_listing()))
        .bind(("127.0.0.1", 4000))?
        .run()
        .await
} 
