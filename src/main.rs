use actix_web::{middleware, post, web, App, HttpResponse, HttpServer, Result};
use env_logger::Env;
use futures::channel::mpsc::Sender;
use futures::lock::Mutex;
use futures::{FutureExt, SinkExt, StreamExt};
use log::{error, info};
use std::process::{Command, Stdio};

struct Channel {
    pub sender: Mutex<Sender<()>>,
}

#[post("/ddl")]
async fn download(data: web::Data<Channel>) -> Result<HttpResponse> {
    info!("Sending download request");
    let mut sender = data.sender.lock().await;
    sender.send(()).await.unwrap();

    Ok(HttpResponse::Ok().finish())
}

fn health() -> HttpResponse {
    HttpResponse::Ok().body("OK\n")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let (sender, mut receiver) = futures::channel::mpsc::channel::<()>(100);
    let mut closer = sender.clone();
    let channel = web::Data::new(Channel {
        sender: Mutex::new(sender),
    });

    let server = HttpServer::new(move || {
        App::new()
            .app_data(channel.clone())
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .service(download)
            .service(web::resource("/health").to(health))
    })
        .bind("0.0.0.0:8877")?
        .run()
        .map(|r| {
            closer.close_channel();
            r
        });

    let runner = async move {
        while let Some(_) = receiver.next().await {
            info!("Initiating rclone download");
            let mut process = Command::new("rclone")
                .args(&["copy", "putio:dl", "/home/pi/share/2watch/dl", "-P"])
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("failed running ddl");

            match process.wait() {
                Ok(_) => info!("Finished download"),
                Err(e) => error!("Error downloading: {}", e),
            }
        }
    };

    let (r, _) = futures::future::join(server, runner).await;
    r
}
