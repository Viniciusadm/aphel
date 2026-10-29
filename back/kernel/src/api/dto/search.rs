use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use crate::application::search::{SearchGroup, SearchHit, SearchRequest, SearchResult};

#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct SearchParams {
    #[param(example = "+55 11 99999", min_length = 2, max_length = 100)]
    pub q: String,
    #[param(example = "contact,deal")]
    pub types: Option<String>,
    #[param(minimum = 1, maximum = 20)]
    pub limit: Option<u32>,
}

impl From<SearchParams> for SearchRequest {
    fn from(params: SearchParams) -> Self {
        Self {
            q: params.q,
            types: params.types,
            limit: params.limit,
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchResponse {
    pub query: String,
    pub groups: Vec<SearchGroupResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchGroupResponse {
    #[schema(example = "contact")]
    pub record_type: String,
    pub total: u64,
    pub items: Vec<SearchHitResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SearchHitResponse {
    pub id: String,
    #[schema(example = "contact")]
    pub record_type: String,
    #[schema(example = "Maria Silva")]
    pub title: String,
    #[schema(example = "maria@acme.com.br")]
    pub subtitle: Option<String>,
    #[schema(example = 4.2)]
    pub score: f64,
}

impl From<SearchHit> for SearchHitResponse {
    fn from(hit: SearchHit) -> Self {
        Self {
            id: hit.id.to_string(),
            record_type: hit.record_type.as_str().to_owned(),
            title: hit.title,
            subtitle: hit.subtitle,
            score: hit.score,
        }
    }
}

impl From<SearchGroup> for SearchGroupResponse {
    fn from(group: SearchGroup) -> Self {
        Self {
            record_type: group.record_type.as_str().to_owned(),
            total: group.total,
            items: group.items.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<SearchResult> for SearchResponse {
    fn from(result: SearchResult) -> Self {
        Self {
            query: result.query,
            groups: result.groups.into_iter().map(Into::into).collect(),
        }
    }
}
