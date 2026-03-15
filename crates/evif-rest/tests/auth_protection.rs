use evif_core::{EvifPlugin, RadixMountTable};
use evif_plugins::MemFsPlugin;
use evif_rest::{create_routes_with_auth, RestAuthState};
use std::sync::Arc;
use tokio::net::TcpListener;

async fn spawn_server(app: axum::Router) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();

    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve");
    });

    let base = format!("http://127.0.0.1:{}", port);
    let client = reqwest::Client::new();
    for _ in 0..50 {
        if let Ok(res) = client.get(format!("{}/api/v1/health", base)).send().await {
            if res.status().is_success() {
                return base;
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(25)).await;
    }

    panic!("server did not become ready in time");
}

#[tokio::test]
async fn test_protected_write_route_requires_api_key() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new()) as Arc<dyn EvifPlugin>;
    mount_table.mount("/mem".to_string(), mem).await.unwrap();

    let auth_state = Arc::new(RestAuthState::from_api_keys(
        vec!["write-key".to_string()],
        vec!["admin-key".to_string()],
    ));
    let base = spawn_server(create_routes_with_auth(mount_table, auth_state.clone())).await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("{}/api/v1/files", base))
        .json(&serde_json::json!({ "path": "/mem/protected.txt" }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::UNAUTHORIZED);

    let events = auth_state.audit_events();
    assert!(events.iter().any(|event| {
        event.event_type == evif_auth::AuditEventType::AuthenticationFailed
            && event.details.contains("/api/v1/files")
            && event.details.contains("missing credentials")
    }));
}

#[tokio::test]
async fn test_write_api_key_can_write_protected_route() {
    let mount_table = Arc::new(RadixMountTable::new());
    let mem = Arc::new(MemFsPlugin::new());
    mount_table
        .mount("/mem".to_string(), mem.clone() as Arc<dyn EvifPlugin>)
        .await
        .unwrap();

    let auth_state = Arc::new(RestAuthState::from_api_keys(
        vec!["write-key".to_string()],
        vec!["admin-key".to_string()],
    ));
    let base = spawn_server(create_routes_with_auth(mount_table, auth_state.clone())).await;
    let client = reqwest::Client::new();

    let create_response = client
        .post(format!("{}/api/v1/files", base))
        .header("x-api-key", "write-key")
        .json(&serde_json::json!({ "path": "/mem/secure.txt" }))
        .send()
        .await
        .unwrap();
    assert!(create_response.status().is_success());

    let write_response = client
        .put(format!("{}/api/v1/files?path=/mem/secure.txt", base))
        .header(reqwest::header::AUTHORIZATION, "Bearer write-key")
        .json(&serde_json::json!({ "data": "secure-content", "encoding": null }))
        .send()
        .await
        .unwrap();
    assert!(write_response.status().is_success());

    let stored = mem.read("/secure.txt", 0, 0).await.unwrap();
    assert_eq!(String::from_utf8(stored).unwrap(), "secure-content");

    let events = auth_state.audit_events();
    assert!(events.iter().any(|event| {
        event.event_type == evif_auth::AuditEventType::AccessGranted
            && event.details.contains("/api/v1/files")
            && event.details.contains("scope=write")
    }));
}

#[tokio::test]
async fn test_admin_route_rejects_write_key_and_accepts_admin_key() {
    let mount_table = Arc::new(RadixMountTable::new());
    let auth_state = Arc::new(RestAuthState::from_api_keys(
        vec!["write-key".to_string()],
        vec!["admin-key".to_string()],
    ));
    let base = spawn_server(create_routes_with_auth(mount_table, auth_state.clone())).await;
    let client = reqwest::Client::new();

    let forbidden = client
        .post(format!("{}/api/v1/metrics/reset", base))
        .header("x-api-key", "write-key")
        .send()
        .await
        .unwrap();
    assert_eq!(forbidden.status(), reqwest::StatusCode::FORBIDDEN);

    let granted = client
        .post(format!("{}/api/v1/metrics/reset", base))
        .header("x-api-key", "admin-key")
        .send()
        .await
        .unwrap();
    assert!(granted.status().is_success());

    let events = auth_state.audit_events();
    assert!(events.iter().any(|event| {
        event.event_type == evif_auth::AuditEventType::AccessDenied
            && event.details.contains("/api/v1/metrics/reset")
            && event.details.contains("scope=admin")
    }));
    assert!(events.iter().any(|event| {
        event.event_type == evif_auth::AuditEventType::AccessGranted
            && event.details.contains("/api/v1/metrics/reset")
            && event.details.contains("scope=admin")
    }));
}
