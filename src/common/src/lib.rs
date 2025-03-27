/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */
pub use crate::error::Result;

pub mod error;
pub mod etcd;
pub mod setting;
pub mod spec;

fn open_server(port: u16) -> String {
    format!("{}:{}", crate::setting::get_config().host.ip, port)
}

fn connect_server(port: u16) -> String {
    format!("http://{}:{}", crate::setting::get_config().host.ip, port)
}

pub mod actioncontroller {
    tonic::include_proto!("actioncontroller");

    pub fn open_server() -> String {
        super::open_server(47001)
    }

    pub fn connect_server() -> String {
        super::connect_server(47001)
    }
}

pub mod apiserver {
    pub fn open_rest_server() -> String {
        super::open_server(47099)
    }
}

pub mod filtergateway {
    tonic::include_proto!("filtergateway");

    pub fn open_server() -> String {
        super::open_server(47002)
    }

    pub fn connect_server() -> String {
        super::connect_server(47002)
    }
}

pub mod monitoringclient {
    tonic::include_proto!("monitoringclient");

    pub fn open_server() -> String {
        super::open_server(47003)
    }

    pub fn connect_server() -> String {
        super::connect_server(47003)
    }
}

pub mod nodeagent {
    tonic::include_proto!("nodeagent");

    pub fn open_server() -> String {
        super::open_server(47004)
    }

    pub fn connect_server() -> String {
        super::connect_server(47004)
    }
}

pub mod policymanager {
    tonic::include_proto!("policymanager");

    pub fn open_server() -> String {
        super::open_server(47005)
    }

    pub fn connect_server() -> String {
        super::connect_server(47005)
    }
}

pub mod statemanager {
    tonic::include_proto!("statemanager");

    pub fn open_server() -> String {
        super::open_server(47006)
    }

    pub fn connect_server() -> String {
        super::connect_server(47006)
    }
}
