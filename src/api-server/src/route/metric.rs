// SPDX-License-Identifier: Apache-2.0

use crate::grpc::receiver::metric_notifier::{
    NewContainerInfo, NewContainerList, NewImageList, NewPodInfo, NewPodList,
};
use axum::{routing::get, Json, Router};

pub fn get_route() -> Router {
    Router::new()
        .route("/metric/image", get(list_image))
        .route("/metric/container", get(list_container))
        .route("/metric/pod", get(list_pod))
}

async fn list_image() -> Json<NewImageList> {
    let mut list: Vec<String> = Vec::new();
    let kvs = common::etcd::get_all_with_prefix("metric/image")
        .await
        .unwrap_or_default();
    for kv in kvs {
        let mut each_list: NewImageList = serde_json::from_str(&kv.value).unwrap_or_default();
        list.append(&mut each_list.images);
    }

    Json(NewImageList { images: list })
}

async fn list_container() -> Json<NewContainerList> {
    let mut list: Vec<NewContainerInfo> = Vec::new();
    let kvs = common::etcd::get_all_with_prefix("metric/container")
        .await
        .unwrap_or_default();
    for kv in kvs {
        let mut each_list: NewContainerList = serde_json::from_str(&kv.value).unwrap_or_default();
        list.append(&mut each_list.containers);
    }
    Json(NewContainerList { containers: list })
}

async fn list_pod() -> Json<NewPodList> {
    let mut list: Vec<NewPodInfo> = Vec::new();
    let kvs = common::etcd::get_all_with_prefix("metric/pod")
        .await
        .unwrap_or_default();
    for kv in kvs {
        let mut each_list: NewPodList = serde_json::from_str(&kv.value).unwrap_or_default();
        list.append(&mut each_list.pods);
    }
    Json(NewPodList { pods: list })
}
