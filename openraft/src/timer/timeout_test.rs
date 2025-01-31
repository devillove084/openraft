use std::time::Duration;

use tokio::sync::oneshot;
use tokio::time::sleep;
use tokio::time::Instant;

use crate::timer::timeout::RaftTimer;
use crate::timer::Timeout;

#[async_entry::test(worker_threads = 3)]
async fn test_timeout() -> anyhow::Result<()> {
    tracing::info!("--- set timeout, recv result");
    {
        let (tx, rx) = oneshot::channel();
        let now = Instant::now();
        let _t = Timeout::new(
            || {
                let _ = tx.send(1u64);
            },
            Duration::from_millis(50),
        );

        let res = rx.await?;
        assert_eq!(1u64, res);

        let elapsed = now.elapsed();
        assert!(elapsed < Duration::from_millis(50 + 20));
        assert!(Duration::from_millis(50 - 20) < elapsed);
    }

    tracing::info!("--- update timeout");
    {
        let (tx, rx) = oneshot::channel();
        let now = Instant::now();
        let t = Timeout::new(
            || {
                let _ = tx.send(1u64);
            },
            Duration::from_millis(50),
        );

        // Update timeout to 100 ms after 20 ms, the expected elapsed is 120 ms.
        sleep(Duration::from_millis(20)).await;
        t.update_timeout(Duration::from_millis(100));

        let _res = rx.await?;

        let elapsed = now.elapsed();
        assert!(elapsed < Duration::from_millis(120 + 20));
        assert!(Duration::from_millis(120 - 20) < elapsed);
    }

    tracing::info!("--- update timeout to a lower value wont take effect");
    {
        let (tx, rx) = oneshot::channel();
        let now = Instant::now();
        let t = Timeout::new(
            || {
                let _ = tx.send(1u64);
            },
            Duration::from_millis(50),
        );

        // Update timeout to 10 ms after 20 ms, the expected elapsed is still 50 ms.
        sleep(Duration::from_millis(20)).await;
        t.update_timeout(Duration::from_millis(10));

        let _res = rx.await?;

        let elapsed = now.elapsed();
        assert!(elapsed < Duration::from_millis(50 + 20));
        assert!(Duration::from_millis(50 - 20) < elapsed);
    }

    tracing::info!("--- drop the `Timeout` will cancel the callback");
    {
        let (tx, rx) = oneshot::channel();
        let now = Instant::now();
        let t = Timeout::new(
            || {
                let _ = tx.send(1u64);
            },
            Duration::from_millis(50),
        );

        // Drop the Timeout after 20 ms, the expected elapsed is 20 ms.
        sleep(Duration::from_millis(20)).await;
        drop(t);

        let res = rx.await;
        assert!(res.is_err());

        let elapsed = now.elapsed();
        assert!(elapsed < Duration::from_millis(20 + 10));
        assert!(Duration::from_millis(20 - 10) < elapsed);
    }

    Ok(())
}
