use std::os::unix::fs::MetadataExt;

use futures::StreamExt;
use service_protos::proto_file_service::{
    grpc_file_server::GrpcFile, DeleteFileRequest, DeleteFileResponse, DownloadFileRequest,
    DownloadFileResponse, ListRequest, ListResponse, MoveFileRequest, MoveFileResponse,
    UploadFileRequest, UploadFileResponse,
};
use tokio::io::AsyncWriteExt;
use tonic::{Request, Response, Result, Status};

use common::file;

#[derive(Default, Debug)]
pub struct FileServer {}

#[async_trait::async_trait]
impl GrpcFile for FileServer {
    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        let list_request = request.into_inner();
        let dir = list_request.file_path;
        let mut read_dir = tokio::fs::read_dir(dir).await?;
        let mut list_response = ListResponse {
            file_infos: Vec::new(),
        };
        while let Some(entry) = read_dir.next_entry().await? {
            let mut file_info = service_protos::proto_file_service::FileInfo::default();
            let path = entry.path();
            if let Ok(info) = common::file::FileInfo::new(&path).await {
                if info.is_dir {
                    file_info.set_file_type(service_protos::proto_file_service::FileType::Dir);
                } else {
                    file_info.set_file_type(service_protos::proto_file_service::FileType::File);
                }
                file_info.size = info.size;
                file_info.file_name = info.path;
                list_response.file_infos.push(file_info);
            }
        }
        Ok(Response::new(list_response))
    }

    async fn upload_file(
        &self,
        request: Request<tonic::Streaming<UploadFileRequest>>,
    ) -> Result<Response<UploadFileResponse>, Status> {
        // todo: if file is existed, we should return an exist error.
        let mut stream = request.into_inner();
        let upload_file_request =
            stream
                .message()
                .await?
                .ok_or(Status::from(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "requset is None",
                )))?;
        let file_path = upload_file_request.file_path;
        let file_name = upload_file_request.file_name;
        let mode = upload_file_request.mode;
        let file = std::path::PathBuf::from(file_path.clone()).join(file_name.clone());
        if file.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!("file {} already exists", file.to_str().unwrap()),
            )
            .into());
        }
        let mut f = tokio::fs::OpenOptions::new()
            .create(true)
            .mode(mode)
            .truncate(true)
            .write(true)
            .open(file)
            .await?;
        let _ = f.write(&upload_file_request.content).await?;
        #[allow(unused_variables)]
        let mut write_times: u32 = 1;

        while let Some(upload_file_request) = stream.message().await? {
            let len = f.write(&upload_file_request.content).await?;
            write_times += 1;
            // Reduce the number of flushes and protect disks.
            // Here the disk is written every 100 MB.
            if write_times.is_multiple_of(common::file::FLUSH_TIME as u32) {
                f.flush().await?;
            }
            if len == 0 {
                break;
            }
        }
        f.flush().await?;
        let upload_file_response = UploadFileResponse {
            file_name,
            file_path,
        };
        Ok(tonic::Response::new(upload_file_response))
    }

    async fn download_file(
        &self,
        request: Request<DownloadFileRequest>,
    ) -> Result<Response<futures::stream::BoxStream<'static, Result<DownloadFileResponse>>>, Status>
    {
        let req = request.into_inner();
        let file = std::path::Path::new(&req.file_path).join(&req.file_name);
        let file_parent = file::get_file_parent(&file)?;
        let file_name = file::get_file_name(&file)?;
        let f = tokio::fs::OpenOptions::new()
            .read(true)
            .open(file.clone())
            .await?;
        let mode = f
            .metadata()
            .await
            .map_err(std::io::Error::other)?
            .mode();

        let (sender, receiver) =
            tokio::sync::mpsc::channel::<Result<DownloadFileResponse, Status>>(2);
        let stream = tokio_stream::wrappers::ReceiverStream::new(receiver).boxed();
        let mut rx = common::file::read_file_content(common::file::path_to_string(&file)?).await?;
        tokio::spawn(async move {
            while let Some(content) = rx.recv().await {
                match content {
                    Ok(c) => {
                        if c.is_empty() {
                            break; //EOF
                        }
                        let response = DownloadFileResponse {
                            file_name: file_name.clone(),
                            file_path: file_parent.clone(),
                            mode,
                            content: c,
                        };
                        match sender
                            .send(Ok(response))
                            .await
                            .map_err(std::io::Error::other)
                        {
                            Ok(_) => {}
                            Err(e) => {
                                let error = e.to_string();
                                sender.send(Err(e.into())).await.unwrap_or_default();
                                return Err(std::io::Error::other(error));
                            }
                        }
                    }
                    Err(e) => {
                        sender
                            .send(Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("read {:?} got error: {:?}", file, e),
                            )
                            .into()))
                            .await
                            .unwrap_or_default();
                        break;
                    }
                }
            }
            Ok(())
        });
        Ok(Response::new(stream))
    }

    async fn delete_files(
        &self,
        request: Request<DeleteFileRequest>,
    ) -> Result<Response<DeleteFileResponse>, Status> {
        let files = request.into_inner().file_names;
        let mut result =
            Ok::<Response<DeleteFileResponse>, Status>(Response::new(DeleteFileResponse {}));
        for file in files.iter() {
            let file_path = std::path::Path::new(file);
            if file_path.is_dir() {
                if let Err(e) = tokio::fs::remove_dir_all(file_path).await {
                    result = Err(e.into());
                };
            } else if let Err(e) = tokio::fs::remove_file(file_path).await {
                result = Err(e.into());
            }
        }
        result
    }

    async fn move_files(
        &self,
        request: Request<MoveFileRequest>,
    ) -> Result<Response<MoveFileResponse>, Status> {
        let req = request.into_inner();
        let src_files = req.src_files;
        let mut result =
            Ok::<Response<MoveFileResponse>, Status>(Response::new(MoveFileResponse {}));
        for src_file in src_files {
            let file_name = file::get_file_name(std::path::Path::new(&src_file))?;
            let new_file_name = std::path::Path::new(&req.destination_dir).join(file_name);
            if let Err(e) = tokio::fs::rename(src_file, new_file_name).await {
                result = Err(e.into());
            }
        }
        result
    }
}
