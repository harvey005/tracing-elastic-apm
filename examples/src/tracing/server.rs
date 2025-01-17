use tonic::{transport::Server, Request, Response, Status};
use tower::ServiceBuilder;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

use hello_world::{
    greeter_server::{Greeter, GreeterServer},
};
use tracing::{metadata::LevelFilter, };
use tracing_elastic_apm::middleware::{apm_tracing_layer_grpc, inject_trace_context};
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[derive(Debug, Default)]
pub struct MyGreeter {}

#[tonic::async_trait]
impl Greeter for MyGreeter {
    #[tracing::instrument(fields(elastic = true))]
    async fn say_hello(
        &self,
        request: Request<hello_world::HelloRequest>,
    ) -> Result<Response<hello_world::HelloReply>, Status> {
        tracing::info!("received request");

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }


    
    async fn create_user(
        &self,
        request: Request<hello_world::CreateUserRequest>,
    ) -> Result<Response<hello_world::CreateUserReply>, Status> {
        let (_,_,req_body) = request.into_parts();
        let reply = hello_world::CreateUserReply {
            id: 1024,
            username: req_body.username,
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let apm_layer = tracing_elastic_apm::new_layer(
        tracing_elastic_apm::apm::config::Config::from_env()
    ).unwrap();

    let filter = EnvFilter::builder().with_default_directive(LevelFilter::INFO.into()).from_env_lossy();
    let stdout = tracing_subscriber::fmt::layer().pretty().compact().with_level(true);
    let subscriber = tracing_subscriber::registry().with(filter).with(stdout).with(apm_layer);
    subscriber.init();

    let addr = "[::1]:50091".parse().unwrap();
    let greeter = MyGreeter::default();

    tracing::info!(message = "Starting GRPC server.", %addr);

    Server::builder()
        .layer(ServiceBuilder::new().layer(apm_tracing_layer_grpc()).map_response(inject_trace_context))
        .add_service(GreeterServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}