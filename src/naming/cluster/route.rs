use std::sync::Arc;

use crate::{
    grpc::PayloadUtils,
    naming::{
        core::{NamingActor, NamingCmd, NamingResult},
        model::{Instance, InstanceUpdateTag},
    },
    raft::network::factory::RaftClusterRequestSender,
};

use super::{
    model::{NamingRouteAddr, NamingRouteRequest, NamingRouterResponse},
    node_manage::{NodeManage, NodeManageRequest},
};

use crate::grpc::handler::NAMING_ROUTE_REQUEST;
use actix::prelude::*;

#[derive(Clone, Debug)]
pub struct NamingRoute {
    naming_addr: Addr<NamingActor>,
    node_manage: Arc<NodeManage>,
    cluster_sender: Arc<RaftClusterRequestSender>,
}

impl NamingRoute {
    pub fn new(
        naming_addr: Addr<NamingActor>,
        node_manage: Arc<NodeManage>,
        cluster_sender: Arc<RaftClusterRequestSender>,
    ) -> Self {
        Self {
            naming_addr,
            node_manage,
            cluster_sender,
        }
    }

    pub async fn update_instance(
        &self,
        instance: Instance,
        tag: Option<InstanceUpdateTag>,
    ) -> anyhow::Result<()> {
        let key = instance.get_service_key();
        match self.node_manage.route_addr(&key).await {
            NamingRouteAddr::Local(_) => {
                let cmd = NamingCmd::Update(instance, tag.clone());
                let res: NamingResult = self.naming_addr.send(cmd).await??;
                if let NamingResult::RewriteToCluster(node_id, instance) = res {
                    let addr = self.node_manage.get_node_addr(node_id).await?;
                    self.do_route_instance(node_id, addr, instance, tag, true)
                        .await?;
                }
            }
            NamingRouteAddr::Remote(cluster_id, addr) => {
                self.do_route_instance(cluster_id, addr, instance, tag, true)
                    .await?;
            }
        };
        Ok(())
    }

    async fn do_route_instance(
        &self,
        cluster_id: u64,
        addr: Arc<String>,
        mut instance: Instance,
        tag: Option<InstanceUpdateTag>,
        is_update: bool,
    ) -> anyhow::Result<()> {
        let req = if is_update {
            NamingRouteRequest::UpdateInstance {
                instance: instance.clone(),
                tag,
            }
        } else {
            NamingRouteRequest::RemoveInstance {
                instance: instance.clone(),
            }
        };
        let request = serde_json::to_string(&req).unwrap_or_default();
        let payload = PayloadUtils::build_payload(NAMING_ROUTE_REQUEST, request);
        let resp_payload = self.cluster_sender.send_request(addr, payload).await?;
        let body_vec = resp_payload.body.unwrap_or_default().value;
        let _: NamingRouterResponse = serde_json::from_slice(&body_vec)?;

        //路由在其它节点后，立即同步本节点
        if is_update {
            if instance.client_id.is_empty() && cluster_id > 0 {
                instance.client_id = Arc::new(format!("{}_G", &cluster_id));
            }
            self.node_manage
                .inner_node_manage
                .do_send(NodeManageRequest::AddClientId(
                    cluster_id,
                    instance.client_id.clone(),
                ));
            instance.from_cluster = cluster_id;
            let cmd = NamingCmd::Update(instance, None);
            self.naming_addr.do_send(cmd);
        } else {
            instance.from_cluster = cluster_id;
            let cmd = NamingCmd::Delete(instance);
            self.naming_addr.do_send(cmd);
        }

        Ok(())
    }

    pub async fn delete_instance(&self, instance: Instance) -> anyhow::Result<()> {
        let key = instance.get_service_key();
        match self.node_manage.route_addr(&key).await {
            NamingRouteAddr::Local(_) => {
                let cmd = NamingCmd::Delete(instance);
                let _: NamingResult = self.naming_addr.send(cmd).await??;
            }
            NamingRouteAddr::Remote(cluster_id, addr) => {
                self.do_route_instance(cluster_id, addr, instance, None, false)
                    .await?;
            }
        };
        Ok(())
    }
}
