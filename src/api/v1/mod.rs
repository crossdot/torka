//! `/api/v1` routes.

use axum::{
    Json, Router,
    extract::{Query, State, rejection::QueryRejection},
    routing::get,
};

use crate::{
    error::{ApiResult, AppError},
    search::model::{SearchQuery, SearchResponse},
    state::AppState,
};

/// `/api/v1` scope. Phase 3 adds `/sources/{id}` and `/playback/...`; unknown
/// paths return the unified JSON 404.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/search", get(search))
        .fallback(not_found)
}

#[derive(serde::Deserialize)]
struct SearchParams {
    q: String,
    year: Option<u16>,
}

async fn search(
    State(state): State<AppState>,
    query: Result<Query<SearchParams>, QueryRejection>,
) -> ApiResult<Json<SearchResponse>> {
    let Query(params) = query.map_err(|e| AppError::validation(e.to_string()))?;
    let query = SearchQuery::normalize(params.q, params.year).map_err(AppError::validation)?;
    let response = state.search.search(&query).await;
    Ok(Json(response))
}

async fn not_found(_state: State<AppState>) -> AppError {
    AppError::not_found("route not found")
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    use super::*;
    use crate::{
        config::Config,
        search::{
            SearchService,
            fake::FakeProvider,
            registry::{AnyProvider, ProviderRegistry},
        },
    };

    fn test_state() -> AppState {
        let registry =
            ProviderRegistry::default().with_provider(AnyProvider::Fake(FakeProvider::new("fake")));
        let search = Arc::new(SearchService::new(registry, Duration::from_secs(5), 4));
        AppState {
            config: Config::default(),
            search,
        }
    }

    async fn body_json(req: Request<Body>) -> (StatusCode, serde_json::Value) {
        let resp = crate::api::router(test_state()).oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = to_bytes(resp.into_body(), 1024 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    #[tokio::test]
    async fn search_returns_items_and_provider_status() {
        let req = Request::builder()
            .uri("/api/v1/search?q=matrix&year=2020")
            .body(Body::empty())
            .unwrap();
        let (status, json) = body_json(req).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
        assert_eq!(json["items"][0]["title"], "matrix [1080p]");
        assert_eq!(json["providers"][0]["status"], "ok");
        assert_eq!(json["providers"][0]["error_code"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn search_rejects_empty_query_with_validation_error() {
        let req = Request::builder()
            .uri("/api/v1/search?q=")
            .body(Body::empty())
            .unwrap();
        let (status, json) = body_json(req).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(json["error"]["code"], "VALIDATION_ERROR");
    }

    #[tokio::test]
    async fn unknown_api_path_returns_json_404() {
        let req = Request::builder()
            .uri("/api/v1/nope")
            .body(Body::empty())
            .unwrap();
        let (status, json) = body_json(req).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(json["error"]["code"], "NOT_FOUND");
    }
}
