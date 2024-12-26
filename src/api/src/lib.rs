/*
 * SPDX-FileCopyrightText: Copyright 2024 LG Electronics Inc.
 * SPDX-License-Identifier: Apache-2.0
 */

pub mod proto {
    pub mod apiserver {
        tonic::include_proto!("apiserver");
        pub mod metric_notifier {
            tonic::include_proto!("apiserver.metric_notifier");
        }
    }
    pub mod gateway {
        tonic::include_proto!("gateway");
    }
    pub mod statemanager {
        tonic::include_proto!("statemanager");
    }
    pub mod constants {
        tonic::include_proto!("constants");
    }
}
