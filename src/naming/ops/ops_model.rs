use std::sync::Arc;

use serde::{Serialize, Deserialize};

use crate::naming::{service_index::ServiceQueryParam, service::ServiceInfoDto};

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsServiceQueryListRequest {
    pub page_no:Option<usize>,
    pub page_size:Option<usize>,
    pub namespace_id:Option<Arc<String>>,
    pub group_name_param:Option<String>,
    pub service_name_param:Option<String>,
    pub access_token:Option<String>,
}

impl OpsServiceQueryListRequest {
    pub fn to_param(self) -> anyhow::Result<ServiceQueryParam> {
        let limit = self.page_size.unwrap_or(0xffff_ffff);
        let offset = (self.page_no.unwrap_or(1)-1)*limit;
        let mut param=ServiceQueryParam {
            limit:limit,
            offset:offset,
            ..Default::default()
        };
        if let Some(namespace_id) = self.namespace_id {
            param.namespace_id = Some(namespace_id);
        }
        if let Some(group_name_param) = self.group_name_param {
            param.like_group = Some(group_name_param);
        }
        if let Some(service_name_param) = self.service_name_param {
            param.like_service= Some(service_name_param);
        }
        Ok(param)
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsServiceDto {
    pub name: Option<Arc<String>>,
    pub group_name: Option<Arc<String>>,
    pub cluster_count: Option<u64>,
    pub ip_count: Option<u64>,
    pub healthy_instance_count: Option<u64>,
    pub trigger_flag: Option<bool>,
}

impl From<ServiceInfoDto> for OpsServiceDto {
    fn from(value: ServiceInfoDto) -> Self {
        Self { name: Some(value.service_name)
            , group_name: Some(value.group_name)
            , cluster_count: Some(value.cluster_count as u64)
            , ip_count: Some(value.instance_size as u64)
            , healthy_instance_count: Some(value.healthy_instance_size as u64)
            , trigger_flag: Some(value.trigger_flag) 
        }
    }
}

#[derive(Debug,Serialize,Deserialize,Default)]
#[serde(rename_all = "camelCase")]
pub struct OpsServiceOptQueryListResponse{
    pub count:u64,
    pub service_list:Vec<OpsServiceDto>,
}

impl OpsServiceOptQueryListResponse {
    pub fn new(count:u64,service_list:Vec<OpsServiceDto>) -> Self {
        Self { count, service_list}
    }
}


