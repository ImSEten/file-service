const IP: &str = "[::]";
const PORT: u16 = 10086;
const PROTOCOL: &str = "grpc";
#[derive(clap::Parser)]
#[command(name = "FileServer")]
#[command(about = "FileServer is my own file server", long_about = None)]
pub struct Flags {
    /// server listening ip addr
    #[arg(long, help = "server test", default_value = "test")]
    pub test: String,

    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// 子命令枚举
#[derive(clap::Subcommand)]
pub enum Commands {
    /// 启动服务
    #[command(name = "start", about = "Start the server")]
    Start {
        /// server listening ip addr
        #[arg(long, help = "server listening ip addr", default_value = IP)]
        ip: String,

        /// server listening ip port
        #[arg(short, long, default_value_t = PORT, help = "server listening ip port")]
        port: u16,

        /// server start type
        #[arg(long,default_value = PROTOCOL, help = "server start protocol include 'grpc http_actix http_axum'")]
        protocol: String,

        /// root directory for file service
        #[arg(long, default_value = "/", help = "root directory for file service")]
        root_dir: String,
    },
    /// 停止服务
    #[command(name = "stop", about = "Stop the server")]
    Stop,
}
