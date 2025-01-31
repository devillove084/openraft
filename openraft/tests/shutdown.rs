use std::sync::Arc;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::Result;
use fixtures::RaftRouter;
use maplit::btreeset;
use openraft::Config;
use openraft::ServerState;

use crate::fixtures::init_default_ut_tracing;

#[macro_use]
mod fixtures;

/// Cluster shutdown test.
///
/// What does this test do?
///
/// - this test builds upon the `initialization` test.
/// - after the cluster has been initialize, it performs a shutdown routine on each node, asserting that the shutdown
///   routine succeeded.
#[async_entry::test(worker_threads = 8, init = "init_default_ut_tracing()", tracing_span = "debug")]
async fn initialization() -> Result<()> {
    // Setup test dependencies.
    let config = Arc::new(Config::default().validate()?);
    let mut router = RaftRouter::new(config.clone());
    router.new_raft_node(0);
    router.new_raft_node(1);
    router.new_raft_node(2);

    let mut log_index = 0;

    // Assert all nodes are in learner state & have no entries.
    router.wait_for_log(&btreeset![0, 1, 2], None, timeout(), "empty").await?;
    router.wait_for_state(&btreeset![0, 1, 2], ServerState::Learner, timeout(), "empty").await?;
    router.assert_pristine_cluster();

    // Initialize the cluster, then assert that a stable cluster was formed & held.
    tracing::info!("--- initializing cluster");
    router.initialize_from_single_node(0).await?;
    log_index += 1;

    router.wait_for_log(&btreeset![0, 1, 2], Some(log_index), None, "init").await?;
    router.assert_stable_cluster(Some(1), Some(1));

    tracing::info!("--- performing node shutdowns");
    {
        let (node0, _) = router.remove_node(0).ok_or_else(|| anyhow!("failed to find node 0 in router"))?;
        node0.shutdown().await?;

        let (node1, _) = router.remove_node(1).ok_or_else(|| anyhow!("failed to find node 1 in router"))?;
        node1.shutdown().await?;

        let (node2, _) = router.remove_node(2).ok_or_else(|| anyhow!("failed to find node 2 in router"))?;
        node2.shutdown().await?;
    }

    Ok(())
}

fn timeout() -> Option<Duration> {
    Some(Duration::from_millis(1000))
}

#[async_entry::test(worker_threads = 8, init = "init_default_ut_tracing()", tracing_span = "debug")]
async fn panic_node() -> Result<()> {
    // Setup test dependencies.
    let config = Arc::new(Config::default().validate()?);
    let mut router = RaftRouter::new(config.clone());
    router.new_raft_node(0);
    router.new_raft_node(1);
    router.new_raft_node(2);

    let mut log_index = 0;

    // Assert all nodes are in learner state & have no entries.
    router.wait_for_log(&btreeset![0, 1, 2], None, timeout(), "empty").await?;
    router.wait_for_state(&btreeset![0, 1, 2], ServerState::Learner, timeout(), "empty").await?;
    router.assert_pristine_cluster();

    // Initialize the cluster, then assert that a stable cluster was formed & held.
    tracing::info!("--- initializing cluster");
    router.initialize_from_single_node(0).await?;
    log_index += 1;

    router.wait_for_log(&btreeset![0, 1, 2], Some(log_index), None, "init").await?;
    router.assert_stable_cluster(Some(1), Some(1));

    router.external_request(0, |_s, _sto, _net| {
        panic!("foo");
    });

    router.assert_stable_cluster(Some(1), Some(1));

    Ok(())
}

#[async_entry::test(worker_threads = 8, init = "init_default_ut_tracing()", tracing_span = "debug")]
async fn fatal_shutdown() -> Result<()> {
    // Setup test dependencies.
    let config = Arc::new(Config::default().validate()?);
    let mut router = RaftRouter::new(config.clone());
    router.new_raft_node(0);
    router.new_raft_node(1);
    router.new_raft_node(2);

    let mut log_index = 0;

    // Assert all nodes are in learner state & have no entries.
    router.wait_for_log(&btreeset![0, 1, 2], None, timeout(), "empty").await?;
    router.wait_for_state(&btreeset![0, 1, 2], ServerState::Learner, timeout(), "empty").await?;
    router.assert_pristine_cluster();

    // Initialize the cluster, then assert that a stable cluster was formed & held.
    tracing::info!("--- initializing cluster");
    router.initialize_from_single_node(0).await?;
    log_index += 1;
    router.wait_for_log(&btreeset![0, 1, 2], Some(log_index), None, "init").await?;

    router.assert_stable_cluster(Some(1), Some(1));
    let leader = router.leader().expect("leader not found");
    let client = router.clone();

    let _h = tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let (removed_node, _) = router.remove_node(leader).expect("node is not found");
        removed_node.shutdown().await
    });

    loop {
        let res = client.client_request(leader, "0", 1).await;
        log_index += 1;
        match res {
            Ok(_response) => {}
            Err(err) => {
                assert_eq!(err.to_string(), "raft stopped");
                break;
            }
        }
    }

    Ok(())
}
