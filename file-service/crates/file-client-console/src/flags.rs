use std::num::NonZero;

pub const IP: &str = "127.0.0.1";
pub const PORT: u16 = 10086;

#[derive(clap::Parser)]
#[command(name = "FileClient")]
#[command(about = "FileClient is my own file server's client", long_about = None)]
pub struct Flags {
    /// server listening ip addr
    #[arg(long, help = "server listening ip addr", default_value = IP)]
    pub ip: String,

    /// server listening ip port
    #[arg(short, long, default_value_t = PORT, help = "server listening ip port")]
    pub port: u16,

    /// The maximum number of simultaneous uploads
    #[arg(long, default_value_t = std::thread::available_parallelism().unwrap_or_else(|_| {println!("cannot get cpu nums, use 1"); NonZero::new(1).expect("number 1 cannot into NoneZero")}).into(), help = "The maximum number of simultaneous. Default is the number of CPU cores.")]
    pub max_simultaneous: usize,

    /// 子命令
    #[command(subcommand)]
    pub command: Option<Commands>,
}

/// 子命令枚举
#[derive(clap::Subcommand)]
pub enum Commands {
    #[command(
        name = "file",
        about = "file subcommand, to operate the file in server"
    )]
    File {
        #[command(subcommand)]
        command: Option<FileCommand>,
    },
}

#[derive(clap::Subcommand)]
pub enum FileCommand {
    #[command(name = "list", about = "list files in server")]
    List {
        /// remote_dir in remote, list all the files in this remote_dir.
        #[arg(
            name = "remote-dir",
            long,
            help = "remote_dir is the dir in server.",
            required = true
        )]
        remote_dir: String,
    },
    #[command(name = "upload-file", about = "upload files to server")]
    UploadFiles {
        /// server listening ip port
        #[arg(
            name = "local-file",
            long,
            help = "the local file, can be abs or relative. This can be call several times, like --local-file file_A --local-file file_B --local-file file_C",
            required = true
        )]
        local_files: Vec<String>,
        /// server listening ip port
        #[arg(
            name = "remote-dir",
            long,
            help = "remote_dir is the dir in server, the file_name is the same as local",
            required = true
        )]
        remote_dir: String,
    },
    #[command(name = "download-file", about = "download files from server")]
    DownloadFile {
        /// server listening ip port
        #[arg(
            name = "remote-file",
            long,
            help = "remote-file, must be abs path, contain the file name. This can be call several times, like --remote-file /xxx/file_A --remote-file /xxx/file_B --remote-file /xxx/file_C",
            required = true
        )]
        remote_files: Vec<String>,

        #[arg(
            name = "local-dir",
            long,
            help = "local-dir is the dir in localhost, the file_name is the same as remote",
            required = true
        )]
        local_dir: String,
    },
    #[command(name = "delete-file", about = "delete files from server")]
    DeleteFile {
        #[arg(
            name = "remote-file",
            long,
            help = "remote-file, must be abs path, contain the file name. This can be call several times, like --remote-file /xxx/file_A --remote-file /xxx/file_B --remote-file /xxx/file_C",
            required = true
        )]
        remote_files: Vec<String>,
    },

    #[command(
        name = "move-file",
        about = "move files from src to dest dir in server"
    )]
    MoveFile {
        #[arg(
            name = "src-file",
            long,
            help = "src-file, must be abs path, contain the file name. This can be call several times, like --remote-file /xxx/file_A --remote-file /xxx/file_B --remote-file /xxx/file_C",
            required = true
        )]
        src_files: Vec<String>,

        #[arg(
            name = "dest-dir",
            long,
            help = "dest-dir is the dest dir in server, move file to the dest-dir",
            required = true
        )]
        destination_dir: String,
    },
}
