use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

const DEFAULT_PER_PAGE: u32 = 20;
const MAX_PER_PAGE: u32 = 100;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
#[into_params(parameter_in = Query)]
pub struct PageParams {
    #[serde(default = "one")]
    pub page: u32,

    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

fn one() -> u32 {
    1
}

fn default_per_page() -> u32 {
    DEFAULT_PER_PAGE
}

impl Default for PageParams {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: DEFAULT_PER_PAGE,
        }
    }
}

impl PageParams {
    /// Always limit the values: a `perPage=100000` sent by the
    /// client must never reach the database.
    pub fn normalized(self) -> Self {
        Self {
            page: self.page.max(1),
            per_page: self.per_page.clamp(1, MAX_PER_PAGE),
        }
    }

    pub fn limit(&self) -> i64 {
        self.normalized().per_page as i64
    }

    pub fn offset(&self) -> i64 {
        let n = self.normalized();
        ((n.page - 1) as i64) * (n.per_page as i64)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Page<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: u32,
    pub per_page: u32,
    pub has_next: bool,
}

impl<T> Page<T> {
    pub fn new(items: Vec<T>, total: i64, params: PageParams) -> Self {
        let p = params.normalized();
        let seen = p.offset() + items.len() as i64;
        Self {
            items,
            total,
            page: p.page,
            per_page: p.per_page,
            has_next: seen < total,
        }
    }
}
