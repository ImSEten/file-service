use std::process::exit;

use actix_web::{web, App, HttpServer};
use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, post},
    Router,
};
use clap::{CommandFactory, Parser};
use common::server::ServerInterface;
use common::Result;

use env_logger::Env;
use log::{error, info};
use tonic::transport::Server;
mod flags;

use service_protos::proto_file_service;
use tower_http::limit::RequestBodyLimitLayer;

pub struct FileServer {
    ip: String,
    port: u16,
}

impl FileServer {
    pub fn new(ip: String, port: u16) -> Self {
        FileServer { ip, port }
    }
}

struct GrpcRequest {}
struct GrpcResponse {}

struct HttpAxumRequest {}
struct HttpAxumResponse {}

struct HttpActixRequest {}
struct HttpActixResponse {}

#[tonic::async_trait]
impl ServerInterface<HttpAxumRequest, HttpAxumResponse> for FileServer {
    async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.ip, self.port);
        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .expect("cannot parse addr");
        info!("Http Server is listening to {}:{}", self.ip, self.port);
        let app = Router::new()
            .route("/", get(http_service::file_server::index_axum))
            .route("/file", get(http_service::file_server::index_axum))
            .route("/file/", get(http_service::file_server::index_axum))
            .route(
                "/file/{*directory}",
                get(http_service::file_server::index_axum),
            )
            .route(
                "/download/{*directory}",
                get(http_service::file_server::download_file_axum),
            )
            .route(
                "/list/{*directory}",
                get(http_service::file_server::list_axum),
            )
            .route(
                "/upload/{*directory}",
                post(http_service::file_server::upload_file_axum),
            )
            .layer(DefaultBodyLimit::disable())
            .layer(RequestBodyLimitLayer::new(512 * 1024 * 1024))
            .route(
                "/merge/{*directory}",
                post(http_service::file_server::merge_file_axum),
            )
            .route(
                "/delete/{*directory}",
                delete(http_service::file_server::delete_file_axum),
            )
            .fallback(http_service::file_server::not_found_axum);
        // 启动服务
        axum::serve(listener, app.into_make_service())
            .await
            .unwrap();
        Ok(())
    }
    async fn stop(&self) -> Result<()> {
        todo!()
    }

    async fn stats(&self) -> Result<()> {
        todo!()
    }

    async fn request(&self, _request: HttpAxumRequest) -> Result<HttpAxumResponse> {
        todo!()
    }
}

#[tonic::async_trait]
impl ServerInterface<HttpActixRequest, HttpActixResponse> for FileServer {
    async fn start(&self) -> Result<()> {
        let addr = format!("{}:{}", self.ip, self.port);
        info!("Http Server is listening to {}:{}", self.ip, self.port);
        let http_task = tokio::spawn(async move {
            HttpServer::new(move || {
                App::new()
                    .service(web::resource("/").to(http_service::file_server::index_actix))
                    .service(
                        web::resource("/list/{path}").to(http_service::file_server::list_actix),
                    )
            })
            .bind(&addr)
            .expect("bind address fail")
            .workers(4)
            .run()
            .await
            .expect("start http failed!");
        });
        let _ = http_task.await;
        Ok(())
    }
    async fn stop(&self) -> Result<()> {
        todo!()
    }

    async fn stats(&self) -> Result<()> {
        todo!()
    }

    async fn request(&self, _request: HttpActixRequest) -> Result<HttpActixResponse> {
        todo!()
    }
}

#[tonic::async_trait]
impl ServerInterface<GrpcRequest, GrpcResponse> for FileServer {
    async fn start(&self) -> Result<()> {
        let addr = (self.ip.clone() + ":" + self.port.to_string().as_str())
            .parse()
            .expect("cannot parse addr");
        info!("Grpc Server is listening to {}:{}", self.ip, self.port);
        let trace_layer =
            tower::ServiceBuilder::new().layer(tower_http::trace::TraceLayer::new_for_grpc());
        let server = grpc_service::file_server::FileServer::default();
        let svc = proto_file_service::grpc_file_server::GrpcFileServer::new(server);
        Server::builder()
            .layer(trace_layer)
            .add_service(svc)
            .serve(addr)
            .await
            .expect("serve server err!");
        Ok(())
    }

    async fn stop(&self) -> Result<()> {
        todo!()
    }

    async fn stats(&self) -> Result<()> {
        todo!()
    }

    async fn request(&self, _request: GrpcRequest) -> Result<GrpcResponse> {
        todo!()
    }
}

fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("debug")).init();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(2)
        .max_blocking_threads(2)
        .build()
        .expect("build tokio runtime error!");
    runtime.block_on(async_main());
}

async fn async_main() {
    let parse_flags = flags::Flags::parse();
    match parse_flags.command {
        Some(flags::Commands::Start { ip, port, protocol }) => {
            let file_server = FileServer::new(ip, port);
            info!("service starting...");
            //todo ! start server also include http
            //may use --type =http to use http service
            match protocol.as_str() {
                "grpc" => {
                    let grpc_server = Box::new(file_server)
                        as Box<dyn ServerInterface<GrpcRequest, GrpcResponse>>;
                    grpc_server
                        .start()
                        .await
                        .expect("start grpc server failed!");
                }
                "http_axum" => {
                    let http_server = Box::new(file_server)
                        as Box<dyn ServerInterface<HttpAxumRequest, HttpAxumResponse>>;
                    http_server
                        .start()
                        .await
                        .expect("start http server failed!");
                }
                "http_actix" => {
                    let http_server = Box::new(file_server)
                        as Box<dyn ServerInterface<HttpActixRequest, HttpActixResponse>>;
                    http_server
                        .start()
                        .await
                        .expect("start http server failed!");
                }
                _ => {
                    error!("unknown  protocol  {}", protocol);
                    exit(-1)
                }
            }

            info!("service exited");
        }
        Some(flags::Commands::Stop) => {
            info!("not implement!")
        }
        None => {
            if let Err(e) = flags::Flags::command().print_help() {
                info!("print_help failed {:?}", e);
            };
        }
    }
}
