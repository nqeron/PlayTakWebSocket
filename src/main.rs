use playtak_ws::server::ExServer;

#[tokio::main]
async fn main(){
    env_logger::init();
    let server : ExServer = ExServer::new(8000);
    server.run().await;
}