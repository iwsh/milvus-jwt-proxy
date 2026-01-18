use std::net::SocketAddr;
use std::sync::Arc;

use hyper::client:                    _ => {
                        // Fallback to HTTP/1.1 or auto-detect
                        let io = TokioIo::new(stream);

                        if let Err(err) = ServerBuilder::new(TokioExecutor::new())
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
                    }yper::server::conn::http2 as server_http2;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::{TokioExecutor, TokioIo};
use hyper_util::server::conn::auto::Builder as ServerBuilder;
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

        info!("Listening on http://{}", addr);
        info!("Upstream URL: {}", self.config.upstream_url);

        loop {
            let (stream, _) = listener.accept().await?;

            let auth_transformer = Arc::clone(&self.auth_transformer);
            let upstream_url = self.config.upstream_url.clone();

            tokio::task::spawn(async move {
                // Detect HTTP/2 prior-knowledge preface. If present, treat as h2; otherwise fall back to HTTP/1.1.
                const PREFACE: &[u8] = b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";
                let mut buf = [0u8; 24];

                match stream.peek(&mut buf).await {
                    Ok(n) if n >= PREFACE.len() && &buf[..PREFACE.len()] == PREFACE => {
                        // h2 prior-knowledge
                        let io = TokioIo::new(stream);

                        if let Err(err) = server_http2::Builder::new(TokioExecutor::new())
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
                            error!("Error serving h2 connection: {:?}", err);
                        }
                    }
                    _ => {
                        // Fallback to HTTP/1.1
                        let io = TokioIo::new(stream);

                        if let Err(err) = Http::new()
                            .http1_only(true)
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
                            error!("Error serving http1 connection: {:?}", err);
                        }
                    }
                }
            });
        }
    }
}

async fn handle_request(
    mut req: Request<hyper::body::Incoming>,
    auth_transformer: Arc<AuthTransformer>,
    upstream_url: String,
) -> Result<Response<hyper::body::Incoming>, hyper::Error> {
    info!("Handling request: {} {}", req.method(), req.uri().path());

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
            // We can't easily return a manual response here because of the hyper::Error return type
            // and hyper::body::Incoming body type. For simplicity in this implementation,
            // we'll let it panic or return a connection error.
            panic!("Invalid upstream URL: {}", e);
        }
    };

    let host = url.host().expect("upstream_url must have a host");
    let port = url.port_u16().unwrap_or(80);
    let addr = format!("{}:{}", host, port);

    let stream = match TcpStream::connect(addr).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to connect to upstream: {}", e);
            panic!("Failed to connect to upstream: {}", e);
        }
    };
    let io = TokioIo::new(stream);

    let (mut sender, conn) = http2::handshake(TokioExecutor::new(), io).await?;

    tokio::task::spawn(async move {
        if let Err(err) = conn.await {
            error!("Upstream connection failed: {:?}", err);
        }
    });

    // 3. Forward request to upstream
    sender.send_request(req).await
}
