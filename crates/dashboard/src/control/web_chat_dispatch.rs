//! Shared web-chat dispatch for conversation start and follow-up messages.

use crate::api::state::AppState;
use crate::control::chat_live_bridge::log_tail_fallback_enabled;
use crate::control::web_chat::WebChatSendResult;
use crate::schema::{InsertEventRequest, SessionDetail};
use axum::http::StatusCode;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;

fn truncate_field(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect()
}

#[allow(clippy::too_many_arguments)]
pub async fn dispatch_web_chat_prompt(
    state: &AppState,
    project_id: &str,
    session_id: &str,
    root: &Path,
    agent_type: Option<&str>,
    prompt: &str,
    prompt_for_chat: &str,
    vision_images: Option<&[crate::control::vision_payload::VisionImagePayload]>,
    text_files: Option<&[crate::control::text_upload::TextFilePayload]>,
    reply_lang: Option<&str>,
    recycled: bool,
    audit_action: &str,
    composer_mode: Option<&str>,
) -> Result<(SessionDetail, WebChatSendResult), (StatusCode, String)> {
    // Keep original attachments for the transcript UI even when OCR strips them for the model.
    let display_vision_images = vision_images;
    let mut model_vision_images = vision_images;
    let mut prompt_for_chat = prompt_for_chat.to_string();
    if let Some(imgs) = model_vision_images {
        if !imgs.is_empty() {
            // OCR may spawn Apple Vision helper (seconds) — keep Tokio workers free.
            let imgs_owned = imgs.to_vec();
            let delivery = match tokio::task::spawn_blocking(move || {
                crate::control::vision_payload::resolve_vision_delivery(&imgs_owned)
            })
            .await
            {
                Ok(inner) => inner,
                Err(e) => {
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("vision delivery task: {e}"),
                    ));
                }
            };
            match delivery {
                Ok(crate::control::vision_payload::VisionDelivery::Native) => {}
                Ok(crate::control::vision_payload::VisionDelivery::OcrText(ocr)) => {
                    prompt_for_chat = crate::control::vision_payload::append_ocr_to_prompt(
                        &prompt_for_chat,
                        &ocr,
                    );
                    // Text-only brain: do not forward raw images to the chat model.
                    model_vision_images = None;
                }
                Err(e) => {
                    return Err((StatusCode::BAD_REQUEST, e.to_string()));
                }
            }
        }
    }
    if let Some(files) = text_files {
        if let Err(e) = crate::control::text_upload::validate_text_payloads(files) {
            return Err((StatusCode::BAD_REQUEST, e.to_string()));
        }
    }
    let prompt_for_chat =
        match crate::control::text_upload::append_to_prompt(&prompt_for_chat, text_files) {
            Ok(p) => p,
            Err(e) => return Err((StatusCode::BAD_REQUEST, e.to_string())),
        };

    if let Ok(evt) = state
        .db
        .insert_event(InsertEventRequest {
            project_id: project_id.to_string(),
            session_id: Some(session_id.to_string()),
            task_id: None,
            agent_id: None,
            event_type: "user_prompt".into(),
            severity: Some("info".into()),
            title: "User prompt".into(),
            body: Some(truncate_field(prompt, 8000)),
            payload: if recycled {
                Some(json!({ "recycled": true }))
            } else {
                None
            },
        })
        .await
    {
        crate::control::web_chat_tail::publish_project_chat_event(&state.events, &evt);
    }

    let embedded = crate::control::chat_runtime::ChatRuntimeHost::enabled();
    let dashboard_url = dashboard_loopback_url(&state.host, state.port);
    let drain = Some(crate::control::message_queue::QueueDrainContext::new(
        Arc::new(state.clone()),
    ));
    let chat_result = if embedded {
        state
            .chat_runtime
            .send(
                state.db.clone(),
                Arc::clone(&state.events),
                &state.web_chat_tail,
                session_id,
                project_id,
                root,
                agent_type,
                prompt,
                &prompt_for_chat,
                display_vision_images,
                model_vision_images,
                reply_lang,
                composer_mode,
                drain,
            )
            .await
            .map_err(|e| {
                if e.downcast_ref::<crate::control::chat_runtime::ChatSendConflict>()
                    .is_some()
                {
                    (StatusCode::CONFLICT, e.to_string())
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            })
    } else {
        state
            .web_chat
            .send(
                state.db.clone(),
                session_id,
                root,
                agent_type,
                &dashboard_url,
                &prompt_for_chat,
                model_vision_images,
                text_files,
                reply_lang,
            )
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
    };

    match chat_result {
        Ok(chat) => {
            if !embedded || log_tail_fallback_enabled() {
                state.web_chat_tail.ensure_tail(
                    Arc::clone(&state.events),
                    session_id,
                    project_id,
                    Path::new(&chat.log_path),
                );
            }
            let mut meta = json!({
                "web_chat": true,
                "web_chat_log_path": chat.log_path,
                "web_chat_pid": chat.pid,
                "embedded_runtime": embedded,
            });
            if recycled {
                meta["recycled"] = json!(true);
                meta["recycled_at"] = json!(chrono::Utc::now().to_rfc3339());
            }
            let _ = state.db.merge_session_metadata(session_id, &meta).await;
            let _ = crate::audit::record_audit(
                &state.db,
                crate::audit::AuditEventInput {
                    project_id: Some(project_id.to_string()),
                    session_id: Some(session_id.to_string()),
                    action: audit_action.into(),
                    risk: "medium".into(),
                    detail: json!({
                        "web_chat": true,
                        "pid": chat.pid,
                        "recycled": recycled,
                        "embedded_runtime": embedded,
                    }),
                },
            )
            .await;
            let session = state
                .db
                .get_session(session_id)
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
                .ok_or_else(|| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "session missing after web chat dispatch".into(),
                    )
                })?;
            Ok((session, chat))
        }
        Err((status, message)) => {
            if status != StatusCode::BAD_REQUEST && status != StatusCode::CONFLICT {
                let _ = state
                    .db
                    .finish_session(
                        session_id,
                        "failed",
                        Some(&format!("Failed to start task: {message}")),
                    )
                    .await;
            }
            Err((status, message))
        }
    }
}

fn dashboard_loopback_url(host: &str, port: u16) -> String {
    let host = match host {
        "0.0.0.0" | "::" => "127.0.0.1",
        other => other,
    };
    format!("http://{host}:{port}")
}
