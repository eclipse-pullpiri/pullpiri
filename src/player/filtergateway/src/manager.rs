// SPDX-License-Identifier: Apache-2.0

use crate::filter::Filter;
use common::filtergateway::Condition;
use lge_dds::DdsData;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};
use tokio::sync::Mutex;

pub struct Manager {
    rx_grpc: Arc<Mutex<Receiver<Condition>>>,
    tx_dds: Sender<DdsData>,
    rx_dds: Arc<Mutex<Receiver<DdsData>>>,
    filters: Arc<Mutex<Vec<Filter>>>,
}

impl Manager {
    pub fn new(rx: Receiver<Condition>) -> Self {
        let (tx_dds, rx_dds) = channel::<DdsData>(10);
        Manager {
            rx_grpc: Arc::new(Mutex::new(rx)),
            tx_dds,
            rx_dds: Arc::new(Mutex::new(rx_dds)),
            filters: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn run(&mut self) {
        lge_dds::run(self.tx_dds.clone()).await;

        let arc_rx_dds = Arc::clone(&self.rx_dds);
        let arc_filters = Arc::clone(&self.filters);
        tokio::spawn(handle_dds(arc_rx_dds, arc_filters));

        let arc_rx_grpc = Arc::clone(&self.rx_grpc);
        let mut rx_grpc = arc_rx_grpc.lock().await;
        while let Some(condition) = rx_grpc.recv().await {
            // if new filter has same name, previous filter is deleted
            self.remove_filter(&condition.name).await;
            match condition.crud.as_str() {
                "CREATE" => self.launch_filter(&condition.name).await,
                //"DELETE" => self.remove_filter(&condition.name).await,
                _ => continue,
            }
        }
    }

    async fn launch_filter(&mut self, name: &str) {
        println!("launch filter {}\n", name);

        let f = Filter::new(name).await;
        let arc_filters = Arc::clone(&self.filters);
        let mut filters = arc_filters.lock().await;
        filters.push(f);
    }

    async fn remove_filter(&mut self, name: &str) {
        println!("remove filter {}\n", name);

        let arc_filters = Arc::clone(&self.filters);
        let mut filters = arc_filters.lock().await;
        let index = filters.iter().position(|f| f.name == name);
        if let Some(i) = index {
            filters.remove(i);
        }
    }
}

async fn handle_dds(
    arc_rx_dds: Arc<Mutex<Receiver<DdsData>>>,
    arc_filters: Arc<Mutex<Vec<Filter>>>,
) {
    let mut rx_dds = arc_rx_dds.lock().await;
    while let Some(data) = rx_dds.recv().await {
        let mut filters = arc_filters.lock().await;
        if filters.is_empty() {
            continue;
        }

        // TODO : apply policy (once, sticky, and so on...)
        //let mut keep: Vec<bool> = Vec::new();
        for filter in filters.iter_mut() {
            if filter.express.is_empty() || data.name != filter.topic {
                continue;
            }

            let result = filter.check(data.clone()).await;
            //keep.push(!result);
            let mut status = "inactive";
            if result {
                status = "active";
                let current_status = common::etcd::get(&format!("scenario/{}/status", filter.name))
                    .await
                    .unwrap_or_default();
                if current_status == "inactive" {
                    // use crate::grpc::sender;
                    // let _ = sender::send(&filter.action_key).await;
                    let action_key = filter.action_key.clone();
                    tokio::spawn(send_action(action_key));
                }
            }
            let _ = common::etcd::put(&format!("scenario/{}/status", filter.name), status).await;
        }
        //let mut iter = keep.iter();
        //filters.retain(|_| *iter.next().unwrap());
    }
}

async fn send_action(action_key: String) {
    use crate::grpc::sender;
    let _ = sender::send(&action_key).await;
}
