use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitWindow {
    pub used_percent: f64,
    pub window_duration_mins: Option<u64>,
    pub resets_at: Option<i64>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Credits {
    #[serde(default)]
    pub unlimited: bool,
    pub balance: Option<String>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitBucket {
    pub limit_id: Option<String>,
    pub limit_name: Option<String>,
    pub primary: Option<LimitWindow>,
    pub secondary: Option<LimitWindow>,
    pub credits: Option<Credits>,
    pub plan_type: Option<String>,
    pub rate_limit_reached_type: Option<serde_json::Value>,
}

pub struct UsageInfo {
    pub buckets: Vec<LimitBucket>,
    pub available_resets: Option<u64>,
    pub ordinary_usage_allowed: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LimitResponse {
    pub rate_limits: Option<LimitBucket>,
    #[serde(default)]
    pub rate_limits_by_limit_id: BTreeMap<String, LimitBucket>,
    pub rate_limit_reset_credits: Option<ResetCredits>,
    pub ordinary_usage_allowed: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResetCredits {
    available_count: Option<u64>,
}

impl UsageInfo {
    pub fn parse(value: serde_json::Value) -> Result<Self, String> {
        let mut response: LimitResponse = serde_json::from_value(value)
            .map_err(|error| format!("Invalid usage-limits response: {error}"))?;
        if let Some(bucket) = response.rate_limits {
            let id = bucket.limit_id.clone().unwrap_or_else(|| "codex".into());
            response.rate_limits_by_limit_id.entry(id).or_insert(bucket);
        }
        let buckets = response
            .rate_limits_by_limit_id
            .into_iter()
            .map(|(id, mut bucket)| {
                if bucket.limit_id.is_none() {
                    bucket.limit_id = Some(id);
                }
                bucket
            })
            .collect();
        Ok(Self {
            buckets,
            available_resets: response
                .rate_limit_reset_credits
                .and_then(|credits| credits.available_count),
            ordinary_usage_allowed: response.ordinary_usage_allowed,
        })
    }
}

impl LimitWindow {
    pub fn remaining_percent(&self) -> f64 {
        (100.0 - self.used_percent).clamp(0.0, 100.0)
    }

    pub fn label(&self) -> String {
        match self.window_duration_mins {
            Some(10080) => "Weekly window".into(),
            Some(minutes) if minutes % 1440 == 0 => format!("{}-day window", minutes / 1440),
            Some(minutes) if minutes % 60 == 0 => format!("{}-hour window", minutes / 60),
            Some(minutes) => format!("{minutes}-minute window"),
            None => "Usage window".into(),
        }
    }

    pub fn reset_label(&self, now: i64) -> String {
        let Some(reset) = self.resets_at else {
            return "Reset time not provided".into();
        };
        let seconds = reset.saturating_sub(now);
        if seconds <= 0 {
            return "Reset time reached — refresh for current limits".into();
        }
        let minutes = (seconds + 59) / 60;
        if minutes >= 1440 {
            format!("Resets in {}d {}h", minutes / 1440, (minutes % 1440) / 60)
        } else if minutes >= 60 {
            format!("Resets in {}h {}m", minutes / 60, minutes % 60)
        } else {
            format!("Resets in {minutes}m")
        }
    }
}
