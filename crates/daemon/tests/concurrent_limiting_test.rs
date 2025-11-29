use av1d_daemon::encode::JobExecutor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// **Feature: av1-reencoder, Property 20: Concurrent job limiting**
/// **Validates: Requirements 19.2**
///
/// For any number of pending jobs, the system should never run more than
/// max_concurrent_jobs simultaneously. This property verifies that the
/// JobExecutor correctly limits concurrent execution.
#[tokio::test]
async fn test_concurrent_job_limiting() {
    // Test with different max_concurrent values
    for max_concurrent in 1..=5 {
        let executor = Arc::new(JobExecutor::new(max_concurrent));
        let concurrent_count = Arc::new(AtomicUsize::new(0));
        let max_observed = Arc::new(AtomicUsize::new(0));

        // Spawn more jobs than the limit
        let num_jobs = max_concurrent * 3;
        let mut handles = Vec::new();

        for job_id in 0..num_jobs {
            let executor_clone = executor.clone();
            let concurrent_count_clone = concurrent_count.clone();
            let max_observed_clone = max_observed.clone();

            let handle = tokio::spawn(async move {
                executor_clone
                    .execute_job(|| async move {
                        // Increment concurrent count
                        let current = concurrent_count_clone.fetch_add(1, Ordering::SeqCst) + 1;

                        // Update max observed
                        max_observed_clone.fetch_max(current, Ordering::SeqCst);

                        // Simulate some work
                        sleep(Duration::from_millis(50)).await;

                        // Decrement concurrent count
                        concurrent_count_clone.fetch_sub(1, Ordering::SeqCst);

                        Ok(PathBuf::from(format!("job-{}", job_id)))
                    })
                    .await
            });

            handles.push(handle);
        }

        // Wait for all jobs to complete
        for handle in handles {
            handle
                .await
                .expect("Job task panicked")
                .expect("Job failed");
        }

        // Verify that we never exceeded the limit
        let max_concurrent_observed = max_observed.load(Ordering::SeqCst);
        assert!(
            max_concurrent_observed <= max_concurrent,
            "Concurrent job limit violated: max_concurrent={}, observed={}",
            max_concurrent,
            max_concurrent_observed
        );

        // Verify that we actually used the available concurrency
        // (at least reached the limit at some point)
        assert!(
            max_concurrent_observed >= max_concurrent.min(num_jobs),
            "Did not utilize available concurrency: max_concurrent={}, observed={}",
            max_concurrent,
            max_concurrent_observed
        );
    }
}
