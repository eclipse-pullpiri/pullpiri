pub mod container;

use hyper::{Body, Client, Error, Uri};
use hyperlocal::{UnixConnector, Uri as UnixUri};

async fn get(path: &str) -> Result<hyper::body::Bytes, Error> {
    let connector = UnixConnector;
    let client = Client::builder().build::<_, Body>(connector);

    // Modify this if you want to run with root authorization
    // or if you have a different socket path.
    // For example, if you run Podman as root, you might use:
    // let socket = "/run/podman/podman.sock";
    // Or if you run it as a user, you might use:
    // let socket = "/run/user/1000/podman/podman.sock
    let socket = "/run/user/1000/podman/podman.sock";
    // let socket = "/var/run/podman/podman.sock";
    let uri: Uri = UnixUri::new(socket, path).into();

    let res = client.get(uri).await?;
    hyper::body::to_bytes(res).await
}
