use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMonthSeriesPoint {
    pub bucket_start: i64,
    pub display_bucket_start: Option<i64>,
    pub total: Option<i64>,
    pub valuable_success: Option<i64>,
    pub valuable_failure: Option<i64>,
    pub other_success: Option<i64>,
    pub other_failure: Option<i64>,
    pub unknown: Option<i64>,
    pub upstream_exhausted: Option<i64>,
    pub new_keys: Option<i64>,
    pub new_quarantines: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardMonthSeries {
    pub current: Vec<DashboardMonthSeriesPoint>,
    pub comparison: Vec<DashboardMonthSeriesPoint>,
}
