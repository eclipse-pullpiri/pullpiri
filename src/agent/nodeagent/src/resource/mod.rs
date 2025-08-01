pub mod container;

use hyper::{Body, Client, Error, Uri};
use hyperlocal::{UnixConnector, Uri as UnixUri};

async fn get(path: &str) -> Result<hyper::body::Bytes, Error> {
    let connector = UnixConnector;
    let client = Client::builder().build::<_, Body>(connector);

    // Modify this if you want to run without root authorization
    // or if you have a different socket path.
    // For example, if you run Podman as root, you might use:
    // let socket = "/var/run/podman/podman.sock";
    // Or if you run it as a user, you might use:
    // let socket = "/run/user/1000/podman/podman.sock
    let socket = "/var/run/podman/podman.sock";
    // let socket = "/var/run/podman/podman.sock";
    let uri: Uri = UnixUri::new(socket, path).into();

    let res = client.get(uri).await?;
    hyper::body::to_bytes(res).await
}

//Unit tets cases
#[cfg(test)]
mod tests {
    use super::get;
    use hyper::body::Bytes;
    use hyper::Error;
    use tokio;

    #[tokio::test]
    async fn test_get_with_valid_path() {
        let result: Result<Bytes, Error> = get("/v1.0/version").await;
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(!bytes.is_empty());
    }
}
