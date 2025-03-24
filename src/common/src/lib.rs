/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
pub use crate::error::Result;

pub mod error;
pub mod etcd;
pub mod setting;
pub mod spec;

pub mod actioncontroller {
    tonic::include_proto!("actioncontroller");

    pub fn open_server() -> String {
        format!("{}:47001", crate::setting::get_config().host.ip)
    }

    pub fn connect_server() -> String {
        format!("http://{}:47001", crate::setting::get_config().host.ip)
    }
}

pub mod apiserver {
    pub fn open_rest_server() -> String {
        format!("{}:47099", crate::setting::get_config().host.ip)
    }
}

pub mod filtergateway {
    tonic::include_proto!("filtergateway");

    pub fn open_server() -> String {
        format!("{}:47002", crate::setting::get_config().host.ip)
    }

    pub fn connect_server() -> String {
        format!("http://{}:47002", crate::setting::get_config().host.ip)
    }
}

pub mod monitoringclient {
    tonic::include_proto!("monitoringclient");

    pub fn open_server() -> String {
        format!("{}:47003", crate::setting::get_config().host.ip)
    }

    pub fn connect_server() -> String {
        format!("http://{}:47003", crate::setting::get_config().host.ip)
    }
}

pub mod nodeagent {
    tonic::include_proto!("nodeagent");

    pub fn open_server() -> String {
        format!("{}:47004", crate::setting::get_config().host.ip)
    }

    pub fn connect_server() -> String {
        format!("http://{}:47004", crate::setting::get_config().host.ip)
    }
}

pub mod policymanager {
    tonic::include_proto!("policymanager");

    pub fn open_server() -> String {
        format!("{}:47005", crate::setting::get_config().host.ip)
    }

    pub fn connect_server() -> String {
        format!("http://{}:47005", crate::setting::get_config().host.ip)
    }
}

pub mod statemanager {
    tonic::include_proto!("statemanager");

    pub fn open_server() -> String {
        format!("{}:47006", crate::setting::get_config().host.ip)
    }

    pub fn connect_server() -> String {
        format!("http://{}:47006", crate::setting::get_config().host.ip)
    }
}
