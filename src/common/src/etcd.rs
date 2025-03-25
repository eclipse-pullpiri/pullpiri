/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

use etcd_client::{Client, DeleteOptions, Error, GetOptions, SortOrder, SortTarget};

pub fn open_server() -> String {
    format!("{}:2379", crate::setting::get_config().host.ip)
}

async fn get_client() -> Result<Client, Error> {
    Client::connect([open_server()], None).await
}

pub struct KV {
    pub key: String,
    pub value: String,
}

pub async fn put(key: &str, value: &str) -> Result<(), Error> {
    let mut client = get_client().await?;
    client.put(key, value, None).await?;
    Ok(())
}

pub async fn get(key: &str) -> Result<String, Error> {
    let mut client = get_client().await?;
    let resp = client.get(key, None).await?;

    if let Some(kv) = resp.kvs().first() {
        Ok(kv.value_str()?.to_string())
    } else {
        Err(etcd_client::Error::InvalidArgs("Key not found".to_string()))
    }
}

pub async fn get_all_with_prefix(key: &str) -> Result<Vec<KV>, Error> {
    let mut client = get_client().await?;
    let option = Some(
        GetOptions::new()
            .with_prefix()
            .with_sort(SortTarget::Create, SortOrder::Ascend),
    );
    let resp = client.get(key, option).await?;

    let vec_kv = resp
        .kvs()
        .iter()
        .map(|kv| KV {
            key: kv.key_str().unwrap_or_default().to_string(),
            value: kv.value_str().unwrap_or_default().to_string(),
        })
        .collect();

    Ok(vec_kv)
}

pub async fn delete(key: &str) -> Result<(), Error> {
    let mut client = get_client().await?;
    client.delete(key, None).await?;
    Ok(())
}

pub async fn delete_all_with_prefix(key: &str) -> Result<(), Error> {
    let mut client = get_client().await?;
    let option = Some(DeleteOptions::new().with_prefix());
    client.delete(key, option).await?;
    Ok(())
}
