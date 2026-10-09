use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use kirzner::{AppState, BusinessStore, router};
use serde_json::{Value, json};
use tower::ServiceExt;

async fn body(response: axum::response::Response) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}
#[tokio::test]
async fn local_access_files_conflicts_and_recovery() {
    let directory = std::env::temp_dir().join(format!("kirzner-http-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let store = BusinessStore::connect(&format!(
        "sqlite://{}",
        directory.join("business.sqlite3").display()
    ))
    .await
    .unwrap();
    let app = router(
        AppState::new(store.clone(), directory.clone()),
        directory.join("frontend"),
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workspace")
                .header("Host", "127.0.0.1:4317")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body(response).await["revision"], 0);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workspace")
                .header("Host", "evil.example")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::FORBIDDEN,
        "Host rejects DNS rebinding"
    );
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/workspace")
                .header("Host", "127.0.0.1:4317")
                .header("Origin", "https://evil.example")
                .header("Content-Type", "application/json")
                .body(Body::from("{}"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let upload = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/files?name=draft.md")
                .header("Host", "127.0.0.1:4317")
                .header("Content-Type", "text/markdown")
                .body(Body::from("Synthetic draft fixture"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(upload.status(), StatusCode::OK);
    let file = body(upload).await;
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/{}", file["reference"].as_str().unwrap()))
                .header("Host", "127.0.0.1:4317")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["Content-Disposition"], "attachment");
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        "Synthetic draft fixture"
    );
    std::fs::write(
        directory.join(file["reference"].as_str().unwrap()),
        "corrupted",
    )
    .unwrap();
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/{}", file["reference"].as_str().unwrap()))
                .header("Host", "127.0.0.1:4317")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.status(),
        StatusCode::SERVICE_UNAVAILABLE,
        "Corrupted referenced content must not be served as the original file"
    );
    for path in [
        "/files/business.sqlite3",
        "/api/forensic",
        "/data/business.sqlite3",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(path)
                    .header("Host", "127.0.0.1:4317")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
    let brief = json!({"product":"Synthetic HTTP test", "customer":"Solo founders", "capabilities":"Draft", "objective":"Test", "time_budget":"1h", "money_budget":"0", "channels":"Owned", "constraints":"No publishing", "materials":[]});
    let mutation = json!({"expected_revision":0,"command":{"type":"save_brief","brief":brief}});
    let request = || {
        Request::builder()
            .method("POST")
            .uri("/api/workspace")
            .header("Host", "127.0.0.1:4317")
            .header("Content-Type", "application/json")
            .body(Body::from(mutation.to_string()))
            .unwrap()
    };
    assert_eq!(
        app.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone().oneshot(request()).await.unwrap().status(),
        StatusCode::CONFLICT,
        "Stale UI revisions cannot overwrite state"
    );
    assert_eq!(store.load_workspace("local").await.unwrap().revision, 1);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/semon")
                .header("Host", "127.0.0.1:4317")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let semon = body(response).await;
    assert_eq!(semon["store_present"], false);
    assert!(
        semon["limitation"]
            .as_str()
            .unwrap()
            .contains("does not verify")
    );
    store.close().await;
    std::fs::remove_dir_all(directory).unwrap();
}
