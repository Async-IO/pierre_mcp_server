use pierre_mcp_server::mcp::progress::ProgressTracker;
use tokio::sync::mpsc;

#[tokio::test]
async fn test_progress_tracking_lifecycle() {
    let tracker = ProgressTracker::new();

    // Start operation
    let token = tracker
        .start_operation("test_operation".to_string(), Some(100.0))
        .await;
    assert!(!token.is_empty());

    // Check initial state
    let (current, total, completed, cancelled) = tracker.get_progress(&token).await.unwrap();
    assert!((current - 0.0).abs() < f64::EPSILON);
    assert_eq!(total, Some(100.0));
    assert!(!completed);
    assert!(!cancelled);

    // Update progress
    tracker
        .update_progress(&token, 50.0, Some("Halfway done".to_string()))
        .await
        .unwrap();
    let (current, _, completed, cancelled) = tracker.get_progress(&token).await.unwrap();
    assert!((current - 50.0).abs() < f64::EPSILON);
    assert!(!completed);
    assert!(!cancelled);

    // Complete operation
    tracker
        .complete_operation(&token, Some("All done!".to_string()))
        .await
        .unwrap();
    let (current, total, completed, cancelled) = tracker.get_progress(&token).await.unwrap();
    let expected = total.unwrap_or(100.0);
    assert!((current - expected).abs() < f64::EPSILON);
    assert!(completed);
    assert!(!cancelled);
}

#[tokio::test]
async fn test_progress_notifications() {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let tracker = ProgressTracker::with_notifications(sender);

    let token = tracker
        .start_operation("notification_test".to_string(), Some(10.0))
        .await;

    // Should receive initial notification
    let notification = receiver.recv().await.unwrap();
    assert_eq!(notification.method, "notifications/progress");
    assert_eq!(notification.params.progress_token, token);
    assert!((notification.params.progress - 0.0).abs() < f64::EPSILON);

    // Update and check notification
    tracker
        .update_progress(&token, 5.0, Some("Progress update".to_string()))
        .await
        .unwrap();
    let notification = receiver.recv().await.unwrap();
    assert!((notification.params.progress - 5.0).abs() < f64::EPSILON);
    assert_eq!(
        notification.params.message,
        Some("Progress update".to_string())
    );
}

#[tokio::test]
async fn test_cleanup_completed() {
    let tracker = ProgressTracker::new();

    let token1 = tracker.start_operation("op1".to_string(), None).await;
    let token2 = tracker.start_operation("op2".to_string(), None).await;

    // Complete one operation
    tracker.complete_operation(&token1, None).await.unwrap();

    // Before cleanup
    assert_eq!(tracker.get_active_operations().await.len(), 2);

    // After cleanup
    tracker.cleanup_completed().await;
    let active = tracker.get_active_operations().await;
    assert_eq!(active.len(), 1);
    assert!(active.contains(&token2));
}

#[tokio::test]
async fn test_operation_cancellation() {
    let tracker = ProgressTracker::new();

    let token = tracker
        .start_operation("cancellable_op".to_string(), Some(50.0))
        .await;

    // Update progress
    tracker
        .update_progress(&token, 25.0, Some("Half way".to_string()))
        .await
        .unwrap();

    // Check not cancelled yet
    assert!(!tracker.is_cancelled(&token).await);

    // Cancel operation
    tracker
        .cancel_operation(&token, Some("User requested cancellation".to_string()))
        .await
        .unwrap();

    // Check cancellation status
    let (current, total, completed, cancelled) = tracker.get_progress(&token).await.unwrap();
    assert!((current - 25.0).abs() < f64::EPSILON); // Progress preserved
    assert_eq!(total, Some(50.0));
    assert!(!completed);
    assert!(cancelled);
    assert!(tracker.is_cancelled(&token).await);

    // Try to update cancelled operation (should fail)
    let result = tracker
        .update_progress(&token, 30.0, Some("Won't work".to_string()))
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("cancelled"));
}

#[tokio::test]
async fn test_cancellation_notifications() {
    let (sender, mut receiver) = mpsc::unbounded_channel();
    let tracker = ProgressTracker::with_notifications(sender);

    let token = tracker
        .start_operation("notify_cancel_test".to_string(), Some(20.0))
        .await;

    // Clear initial notification
    receiver.recv().await.unwrap();

    // Cancel and check notification
    tracker
        .cancel_operation(&token, Some("Test cancellation".to_string()))
        .await
        .unwrap();
    let notification = receiver.recv().await.unwrap();
    assert_eq!(notification.params.progress_token, token);
    assert_eq!(
        notification.params.message,
        Some("Test cancellation".to_string())
    );
}
