// 功能：验证消息分包、粘包和超限拒绝，不依赖应用配置或真实页面。
use super::*;
use tokio::io::AsyncWriteExt;

#[tokio::test]
async fn fragmented_and_consecutive_frames() {
    let (mut writer, mut reader) = tokio::io::duplex(32);
    let task = tokio::spawn(async move {
        for value in [
            serde_json::json!({"中文":"空 格"}),
            serde_json::json!([1, 2, 3]),
        ] {
            let bytes = serde_json::to_vec(&value).unwrap();
            for b in (bytes.len() as u32).to_le_bytes().into_iter().chain(bytes) {
                writer.write_all(&[b]).await.unwrap();
            }
        }
    });
    let first: Value = transport::read(&mut reader).await.unwrap();
    let second: Value = transport::read(&mut reader).await.unwrap();
    assert_eq!(first["中文"], "空 格");
    assert_eq!(second, serde_json::json!([1, 2, 3]));
    task.await.unwrap();
}
#[tokio::test]
async fn oversized_or_incomplete_messages_fail_before_decode() {
    let (mut writer, mut reader) = tokio::io::duplex(32);
    writer.write_u32_le((MAX_MESSAGE + 1) as u32).await.unwrap();
    assert_eq!(
        transport::read::<Value>(&mut reader)
            .await
            .unwrap_err()
            .code,
        "invalidArgument"
    );
    let (mut writer, mut reader) = tokio::io::duplex(32);
    writer.write_u32_le(10).await.unwrap();
    drop(writer);
    assert_eq!(
        transport::read::<Value>(&mut reader)
            .await
            .unwrap_err()
            .code,
        "connectionFailed"
    );
}
#[test]
fn errors_preserve_committed_facts() {
    let response = Response::result(
        "req".into(),
        Some("instance".into()),
        Err(Failure::new("loadFailed", "失败")
            .detail(serde_json::json!({"committed":true,"pageId":"p"}))),
    );
    assert!(!response.ok);
    assert_eq!(response.data["committed"], true);
    assert_eq!(response.error.unwrap().exit_code(), 8);
}

/// 旧凭据、伪造会话、协议不符、禁用状态及旧实例均不能被业务入口接受。
#[test]
fn handshake_rejects_wrong_identity_and_disabled_control() {
    let instance = Instance {
        app_id: "test".into(),
        pid: 1,
        session_id: 2,
        instance_id: "current".into(),
        pipe: "pipe".into(),
        protocol_version: VERSION,
        credential: "secret".into(),
        enabled: true,
    };
    let request = Request {
        protocol_version: VERSION,
        request_id: "r".into(),
        credential: "secret".into(),
        session_id: 2,
        expected_instance: None,
        method: "pages.list".into(),
        params: serde_json::json!({}),
    };
    assert!(request.authorize(&instance, 2, true).is_ok());
    assert_eq!(
        request.authorize(&instance, 3, true).unwrap_err().code,
        "authenticationFailed"
    );
    assert_eq!(
        request.authorize(&instance, 2, false).unwrap_err().code,
        "controlDisabled"
    );
    let mut changed = request.clone();
    changed.credential = "old".into();
    assert_eq!(
        changed.authorize(&instance, 2, true).unwrap_err().code,
        "authenticationFailed"
    );
    let mut changed = request.clone();
    changed.protocol_version += 1;
    assert_eq!(
        changed.authorize(&instance, 2, true).unwrap_err().code,
        "protocolMismatch"
    );
    let mut changed = request;
    changed.expected_instance = Some("old-instance".into());
    assert_eq!(
        changed.authorize(&instance, 2, true).unwrap_err().code,
        "revisionConflict"
    );
}
