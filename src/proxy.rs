use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

use bytes::Bytes;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::client::conn::{http1, http2};
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode, Version};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto as server_auto;
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

use crate::auth::AuthTransformer;
use crate::config::Config;

pub struct Proxy {
    config: Config,
    auth_transformer: Arc<AuthTransformer>,
}

impl Proxy {
    pub fn new(config: Config) -> Self {
        Proxy {
            config,
            auth_transformer: Arc::new(AuthTransformer::new()),
        }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr: SocketAddr = self.config.listen_addr.parse()?;
        let listener = TcpListener::bind(addr).await?;

        info!(%addr, "Listening");
        info!(upstream = %self.config.upstream_url, "Upstream URL");

        loop {
            let (stream, _) = listener.accept().await?;
            let io = TokioIo::new(stream);

            let auth_transformer = Arc::clone(&self.auth_transformer);
            let upstream_url = self.config.upstream_url.clone();

            tokio::task::spawn(async move {
                if let Err(err) = server_auto::Builder::new(TokioExecutor::new())
                    .serve_connection(
                        io,
                        service_fn(move |req| {
                            let auth_transformer = Arc::clone(&auth_transformer);
                            let upstream_url = upstream_url.clone();
                            handle_request(req, auth_transformer, upstream_url)
                        }),
                    )
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }
    }
}

async fn handle_request(
    mut req: Request<hyper::body::Incoming>,
    auth_transformer: Arc<AuthTransformer>,
    upstream_url: String,
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, Infallible> {
    info!(method = %req.method(), path = %req.uri().path(), "Handling request");

    // Log protocol version
    match req.version() {
        Version::HTTP_11 => info!("Protocol: HTTP/1.1"),
        Version::HTTP_2 => info!("Protocol: HTTP/2"),
        v => info!("Protocol: {:?}", v),
    }

    // 1. Rewrite Authorization header
    if let Some(auth_header) = req.headers().get(hyper::header::AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            let transformed = auth_transformer.transform_auth_header(auth_str);
            if transformed != auth_str {
                info!("Transformed authorization header");
                req.headers_mut().insert(
                    hyper::header::AUTHORIZATION,
                    hyper::header::HeaderValue::from_str(&transformed).unwrap(),
                );
            }
        }
    }

    // 2. Prepare upstream request
    let url = match upstream_url.parse::<hyper::Uri>() {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid upstream URL: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                .unwrap());
        }
    };

    let host = url.host().expect("upstream_url must have a host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);

    let stream = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to connect to upstream: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                .unwrap());
        }
    };
    let io = TokioIo::new(stream);

    // Choose upstream client handshake based on incoming request version
    match req.version() {
        Version::HTTP_11 => {
            info!("Using HTTP/1.1 upstream client");
            let (mut sender, conn) = match http1::handshake(io).await {
                Ok(h) => h,
                Err(e) => {
                    error!("HTTP/1.1 handshake failed: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                        .unwrap());
                }
            };

            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    error!("Upstream connection failed: {:?}", err);
                }
            });

            // Forward request to upstream
            match sender.send_request(req).await {
                Ok(resp) => {
                    let (parts, body) = resp.into_parts();
                    let body_bytes = match body.collect().await {
                        Ok(collected) => collected.to_bytes(),
                        Err(_) => Bytes::new(),
                    };
                    let boxed = BoxBody::new(Full::new(body_bytes).map_err(|never| match never {}));

                    let mut builder = Response::builder().status(parts.status);
                    for (k, v) in parts.headers.iter() {
                        builder = builder.header(k, v);
                    }
                    Ok(builder.body(boxed).unwrap())
                }
                Err(e) => {
                    error!("Failed to send request to upstream: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                        .unwrap())
                }
            }
        }
        Version::HTTP_2 => {
            info!("Using HTTP/2 upstream client");
            let (mut sender, conn) = match http2::handshake(TokioExecutor::new(), io).await {
                Ok(h) => h,
                Err(e) => {
                    error!("HTTP/2 handshake failed: {}", e);
                    return Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                        .unwrap());
                }
            };

            tokio::task::spawn(async move {
                if let Err(err) = conn.await {
                    error!("Upstream connection failed: {:?}", err);
                }
            });

            // Forward request to upstream
            match sender.send_request(req).await {
                Ok(resp) => {
                    let (parts, body) = resp.into_parts();
                    let body_bytes = match body.collect().await {
                        Ok(collected) => collected.to_bytes(),
                        Err(_) => Bytes::new(),
                    };
                    let boxed = BoxBody::new(Full::new(body_bytes).map_err(|never| match never {}));

                    let mut builder = Response::builder().status(parts.status);
                    for (k, v) in parts.headers.iter() {
                        builder = builder.header(k, v);
                    }
                    Ok(builder.body(boxed).unwrap())
                }
                Err(e) => {
                    error!("Failed to send request to upstream: {}", e);
                    Ok(Response::builder()
                        .status(StatusCode::BAD_GATEWAY)
                        .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                        .unwrap())
                }
            }
        }
        _ => {
            error!("Unsupported protocol version: {:?}", req.version());
            Ok(Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(BoxBody::new(Empty::new().map_err(|never| match never {})))
                .unwrap())
        }
    }
}
