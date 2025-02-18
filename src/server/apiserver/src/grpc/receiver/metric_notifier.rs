/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use std::collections::HashMap;

use common::apiserver::metric_connection_server::MetricConnection;
use common::apiserver::metric_notifier::{
    ContainerInfo, ContainerList, ImageList, PodContainerInfo, PodInfo, PodList,
};
use common::apiserver::Response;
use tonic::Request;

type GrpcResult = Result<tonic::Response<Response>, tonic::Status>;

#[derive(Default)]
pub struct GrpcMetricServer {}

#[tonic::async_trait]
impl MetricConnection for GrpcMetricServer {
    async fn send_image_list(&self, request: Request<ImageList>) -> GrpcResult {
        //println!("Got a request from {:?}", request.remote_addr());

        let image_list = request.into_inner();
        let node_name = &image_list.node_name;
        let etcd_key = format!("metric/image/{node_name}");
        let new_image_list = NewImageList::from(image_list);
        let json_string = serde_json::to_string(&new_image_list).unwrap();
        //println!("image\n{:#?}", j);
        let _ = common::etcd::put(&etcd_key, &json_string).await;

        Ok(tonic::Response::new(Response {
            resp: true.to_string(),
        }))
    }

    async fn send_container_list(&self, request: Request<ContainerList>) -> GrpcResult {
        //println!("Got a request from {:?}", request.remote_addr());

        let container_list = request.into_inner();
        let node_name = container_list.node_name.clone();
        let etcd_key = format!("metric/container/{node_name}");
        let new_container_list = NewContainerList::from(container_list);
        let json_string = serde_json::to_string(&new_container_list).unwrap();
        //println!("container\n{:#?}", j);

        let _ = common::etcd::put(&etcd_key, &json_string).await;

        Ok(tonic::Response::new(Response {
            resp: true.to_string(),
        }))
    }

    async fn send_pod_list(&self, request: Request<PodList>) -> GrpcResult {
        //println!("Got a request from {:?}", request.remote_addr());

        let pod_list = request.into_inner();
        let node_name = &pod_list.node_name;
        let etcd_key = format!("metric/pod/{node_name}");
        let new_pod_list = NewPodList::from(pod_list);
        let json_string = serde_json::to_string(&new_pod_list).unwrap();
        //println!("pod\n{:#?}", j);

        let _ = common::etcd::put(&etcd_key, &json_string).await;

        Ok(tonic::Response::new(Response {
            resp: true.to_string(),
        }))
    }
}

/*
 * Copied structure for applying serde trait
*/
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
pub struct NewImageList {
    pub images: Vec<String>,
}

impl From<ImageList> for NewImageList {
    fn from(value: ImageList) -> Self {
        NewImageList {
            images: value.images,
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct NewContainerList {
    pub containers: Vec<NewContainerInfo>,
}

#[derive(Deserialize, Serialize)]
pub struct NewContainerInfo {
    pub id: String,
    pub names: Vec<String>,
    pub image: String,
    pub state: HashMap<String, String>,
    pub config: HashMap<String, String>,
    pub annotation: HashMap<String, String>,
}

impl From<ContainerList> for NewContainerList {
    fn from(value: ContainerList) -> Self {
        let nv = value
            .containers
            .into_iter()
            .map(NewContainerInfo::from)
            .collect::<Vec<NewContainerInfo>>();
        NewContainerList { containers: nv }
    }
}

impl From<ContainerInfo> for NewContainerInfo {
    fn from(value: ContainerInfo) -> Self {
        NewContainerInfo {
            id: value.id,
            names: value.names,
            image: value.image,
            state: value.state,
            config: value.config,
            annotation: value.annotation,
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
pub struct NewPodList {
    pub pods: Vec<NewPodInfo>,
}

#[derive(Deserialize, Serialize)]
pub struct NewPodInfo {
    pub id: String,
    pub name: String,
    pub containers: Vec<NewPodContainerInfo>,
    pub state: String,
    pub host_name: String,
    pub created: String,
}

#[derive(Deserialize, Serialize)]
pub struct NewPodContainerInfo {
    pub id: String,
    pub name: String,
    pub state: String,
}

impl From<PodList> for NewPodList {
    fn from(value: PodList) -> Self {
        let nv = value
            .pods
            .into_iter()
            .map(NewPodInfo::from)
            .collect::<Vec<NewPodInfo>>();
        NewPodList { pods: nv }
    }
}

impl From<PodInfo> for NewPodInfo {
    fn from(value: PodInfo) -> Self {
        let nv = value
            .containers
            .into_iter()
            .map(NewPodContainerInfo::from)
            .collect::<Vec<NewPodContainerInfo>>();

        NewPodInfo {
            id: value.id,
            name: value.name,
            containers: nv,
            state: value.state,
            host_name: value.host_name,
            created: value.created,
        }
    }
}

impl From<PodContainerInfo> for NewPodContainerInfo {
    fn from(value: PodContainerInfo) -> Self {
        NewPodContainerInfo {
            id: value.id,
            name: value.name,
            state: value.state,
        }
    }
}
