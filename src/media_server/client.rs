use std::time::Duration;

use reqwest::{RequestBuilder, Url};
use serde::de::DeserializeOwned;

use super::{
    config::Config,
    error::{Error, Result},
    models::{ImageKind, Library, MediaItem, QueryResult, User},
};

#[derive(Debug, Clone)]
pub struct Client {
    config: Config,
    base_url: Url,
    http: reqwest::Client,
}

impl Client {
    /// 创建媒体服务器客户端并统一处理基础 URL、超时和认证请求头。
    pub fn new(mut config: Config) -> Result<Self> {
        config.base_url = config.base_url.trim_end_matches('/').to_string();
        let base_url = Url::parse(&format!("{}/", config.base_url))
            .map_err(|err| Error::InvalidUrl(err.to_string()))?;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout))
            .user_agent(format!("AutoFilm/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(Error::BuildClient)?;

        Ok(Self {
            config,
            base_url,
            http,
        })
    }

    pub fn id(&self) -> &str {
        &self.config.id
    }

    pub fn kind(&self) -> super::Kind {
        self.config.kind
    }

    /// 返回配置用户 ID；未配置时从服务器用户列表中选择首个用户。
    pub async fn resolve_user_id(&self) -> Result<String> {
        if let Some(user_id) = self
            .config
            .user_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            return Ok(user_id.to_string());
        }

        self.get_json::<Vec<User>>("Users", &[])
            .await?
            .into_iter()
            .next()
            .map(|user| user.id)
            .ok_or(Error::MissingUser)
    }

    /// 获取 Jellyfin/Emby 媒体库列表。
    pub async fn libraries(&self) -> Result<Vec<Library>> {
        Ok(self
            .get_json::<QueryResult<Library>>("Library/MediaFolders", &[])
            .await?
            .items)
    }

    /// 获取指定媒体库下的候选媒体项目。
    pub async fn items(
        &self,
        user_id: &str,
        library_id: &str,
        sort_by: &str,
        random_seed: Option<u64>,
        limit: usize,
    ) -> Result<Vec<MediaItem>> {
        let path = format!("Users/{user_id}/Items");
        let mut query = vec![
            ("ParentId", library_id.to_string()),
            ("Recursive", "true".to_string()),
            ("SortBy", sort_by.to_string()),
            ("SortOrder", "Descending".to_string()),
            (
                "IncludeItemTypes",
                "Movie,Series,Episode,MusicAlbum,Audio,MusicVideo,BoxSet".to_string(),
            ),
            ("Limit", limit.to_string()),
            (
                "Fields",
                "SeriesId,ParentBackdropItemId,ParentBackdropImageTags,BackdropImageTags"
                    .to_string(),
            ),
        ];
        if let Some(seed) = random_seed {
            query.push(("RandomSeed", seed.to_string()));
        }
        let query = query
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect::<Vec<_>>();

        Ok(self
            .get_json::<QueryResult<MediaItem>>(&path, &query)
            .await?
            .items)
    }

    /// 下载指定媒体项目的 Primary 或 Backdrop 图片。
    pub async fn download_image(
        &self,
        item_id: &str,
        kind: ImageKind,
        tag: Option<&str>,
    ) -> Result<Vec<u8>> {
        let path = format!("Items/{item_id}/Images/{}", kind.path());
        let query = tag.map(|tag| vec![("tag", tag)]).unwrap_or_default();
        let response = self
            .request(reqwest::Method::GET, &path, &query)?
            .send()
            .await?;
        Ok(Self::success(response).await?.bytes().await?.to_vec())
    }

    /// 使用媒体服务器要求的 Base64 请求体更新媒体库 Primary 图片。
    pub async fn upload_primary_image(&self, library_id: &str, encoded_png: &str) -> Result<()> {
        let path = format!("Items/{library_id}/Images/Primary");
        let response = self
            .request(reqwest::Method::POST, &path, &[])?
            .header(reqwest::header::CONTENT_TYPE, "image/png")
            .body(encoded_png.to_string())
            .send()
            .await?;
        Self::success(response).await?;
        Ok(())
    }

    async fn get_json<T>(&self, path: &str, query: &[(&str, &str)]) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .request(reqwest::Method::GET, path, query)?
            .send()
            .await?;
        Ok(Self::success(response).await?.json().await?)
    }

    fn request(
        &self,
        method: reqwest::Method,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<RequestBuilder> {
        let url = self
            .base_url
            .join(path)
            .map_err(|err| Error::InvalidUrl(err.to_string()))?;
        Ok(self
            .http
            .request(method, url)
            .header("X-Emby-Token", &self.config.api_key)
            .query(&[("api_key", self.config.api_key.as_str())])
            .query(query))
    }

    async fn success(response: reqwest::Response) -> Result<reqwest::Response> {
        if response.status().is_success() {
            return Ok(response);
        }

        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| status.canonical_reason().unwrap_or("").to_string());
        Err(Error::HttpStatus { status, body })
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::mpsc,
        thread,
    };

    use super::*;
    use crate::media_server::Kind;

    fn config(base_url: &str) -> Config {
        Config {
            id: "server".to_string(),
            kind: Kind::Jellyfin,
            base_url: base_url.to_string(),
            api_key: "secret".to_string(),
            user_id: Some("user".to_string()),
            timeout: 30,
        }
    }

    #[test]
    fn normalizes_base_url() {
        let client = Client::new(config("http://localhost:8096/")).unwrap();

        assert_eq!(client.base_url.as_str(), "http://localhost:8096/");
        assert_eq!(client.id(), "server");
    }

    #[tokio::test]
    async fn uses_configured_user_id_without_request() {
        let client = Client::new(config("http://localhost:8096")).unwrap();

        assert_eq!(client.resolve_user_id().await.unwrap(), "user");
    }

    #[tokio::test]
    async fn performs_authenticated_media_server_workflow() {
        let (base_url, requests) = spawn_server(vec![
            Response::json(r#"[{"Id":"user"}]"#),
            Response::json(r#"{"Items":[{"Id":"library","Name":"电影"}]}"#),
            Response::json(
                r#"{"Items":[{"Id":"movie","Type":"Movie","ImageTags":{"Primary":"tag"}}]}"#,
            ),
            Response::bytes("200 OK", "image/png", b"image".to_vec()),
            Response::bytes("204 No Content", "text/plain", Vec::new()),
        ]);
        let mut server_config = config(&base_url);
        server_config.user_id = None;
        let client = Client::new(server_config).unwrap();

        assert_eq!(client.resolve_user_id().await.unwrap(), "user");
        let libraries = client.libraries().await.unwrap();
        let items = client
            .items("user", "library", "DateCreated", None, 50)
            .await
            .unwrap();
        let image = client
            .download_image("movie", ImageKind::Primary, Some("tag"))
            .await
            .unwrap();
        client
            .upload_primary_image("library", "encoded-image")
            .await
            .unwrap();

        assert_eq!(libraries[0].name, "电影");
        assert_eq!(items[0].id, "movie");
        assert_eq!(image, b"image");

        let requests = (0..5).map(|_| requests.recv().unwrap()).collect::<Vec<_>>();
        assert!(requests[0].starts_with("GET /Users?api_key=secret HTTP/1.1"));
        assert!(requests[1].contains("/Library/MediaFolders?api_key=secret"));
        assert!(requests[2].contains("ParentId=library"));
        assert!(requests[2].contains("SortBy=DateCreated"));
        assert!(requests[3].contains("/Items/movie/Images/Primary?api_key=secret&tag=tag"));
        assert!(requests[4].starts_with("POST /Items/library/Images/Primary?api_key=secret"));
        assert!(
            requests[4]
                .to_ascii_lowercase()
                .contains("x-emby-token: secret")
        );
        assert!(requests[4].ends_with("encoded-image"));
    }

    struct Response {
        status: &'static str,
        content_type: &'static str,
        body: Vec<u8>,
    }

    impl Response {
        fn json(body: &str) -> Self {
            Self::bytes("200 OK", "application/json", body.as_bytes().to_vec())
        }

        fn bytes(status: &'static str, content_type: &'static str, body: Vec<u8>) -> Self {
            Self {
                status,
                content_type,
                body,
            }
        }
    }

    fn spawn_server(responses: Vec<Response>) -> (String, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            for response in responses {
                let (mut stream, _) = listener.accept().unwrap();
                let request = read_request(&mut stream);
                sender
                    .send(String::from_utf8_lossy(&request).to_string())
                    .unwrap();
                write!(
                    stream,
                    "HTTP/1.1 {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    response.status,
                    response.content_type,
                    response.body.len()
                )
                .unwrap();
                stream.write_all(&response.body).unwrap();
            }
        });

        (format!("http://{address}"), receiver)
    }

    fn read_request(stream: &mut std::net::TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            let read = stream.read(&mut buffer).unwrap();
            if read == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..read]);
            let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .and_then(|value| value.trim().parse::<usize>().ok())
                })
                .unwrap_or_default();
            if request.len() >= header_end + content_length {
                break;
            }
        }
        request
    }
}
