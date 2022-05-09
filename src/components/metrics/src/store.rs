// Copyright 2022 The Engula Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use lazy_static::lazy_static;
use prometheus::*;

lazy_static! {
    pub static ref STORE_RPC_REQUESTS_TOTAL: IntCounterVec = register_int_counter_vec!(
        "store_rpc_requests_total",
        "Number of QPS for Store service",
        &["op"],
    )
    .unwrap();
    pub static ref STORE_RPC_READ_QPS: IntCounter = STORE_RPC_REQUESTS_TOTAL
        .get_metric_with_label_values(&["read"])
        .unwrap();
    pub static ref STORE_RPC_WRITE_QPS: IntCounter = STORE_RPC_REQUESTS_TOTAL
        .get_metric_with_label_values(&["write"])
        .unwrap();
    pub static ref STORE_RPC_SEAL_QPS: IntCounter = STORE_RPC_REQUESTS_TOTAL
        .get_metric_with_label_values(&["seal"])
        .unwrap();
    pub static ref STORE_RPC_TRUNCATE_QPS: IntCounter = STORE_RPC_REQUESTS_TOTAL
        .get_metric_with_label_values(&["truncate"])
        .unwrap();
    pub static ref STORE_RECEIVED_ENTRIES_TOTAL: IntCounter = register_int_counter!(
        "store_received_entries_total",
        "Number of entries this store received from client",
    )
    .unwrap();
}
